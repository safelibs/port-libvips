#!/usr/bin/env bash
set -euo pipefail

readonly REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly DOCKER_IMAGE="${DOCKER_IMAGE:-ubuntu:24.04}"
readonly JOBS="${JOBS:-$(nproc)}"

docker run --rm -i \
  -e DEBIAN_FRONTEND=noninteractive \
  -e JOBS="${JOBS}" \
  -v "${REPO_ROOT}:/work" \
  -w /work \
  "${DOCKER_IMAGE}" \
  bash -s <<'CONTAINER'
set -euo pipefail

log() {
  printf '\n==> %s\n' "$*"
}

enable_source_repositories() {
  cat >/etc/apt/sources.list.d/ubuntu-src.sources <<'EOF'
Types: deb-src
URIs: http://archive.ubuntu.com/ubuntu
Suites: noble noble-updates noble-backports
Components: main universe restricted multiverse
Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg
EOF
}

install_base_tools() {
  apt-get update
  apt-get install -y --no-install-recommends \
    ca-certificates \
    build-essential \
    cmake \
    dpkg-dev \
    fakeroot \
    git \
    ninja-build \
    pkgconf \
    python3 \
    rsync \
    xauth \
    xvfb
}

load_dependents() {
  mapfile -t DEPENDENTS < <(
    python3 - <<'PY'
import json
from pathlib import Path
import sys

path = Path("/work/dependents.json")
data = json.loads(path.read_text())
packages = [entry["package"] for entry in data.get("dependents", [])]
expected = {"nip2", "photoqt", "ruby-vips"}
if set(packages) != expected:
    sys.exit(f"unsupported dependent set in {path}: {packages}")
print("\n".join(packages))
PY
  )
  log "Testing dependents from dependents.json: ${DEPENDENTS[*]}"
}

build_and_install_original_libvips() {
  log "Installing libvips build dependencies"
  apt-get build-dep -y vips

  rm -rf /tmp/libvips-build
  mkdir -p /tmp/libvips-build
  rsync -a --delete /work/original/ /tmp/libvips-build/source/

  log "Building original libvips Ubuntu packages"
  (
    cd /tmp/libvips-build/source
    export DEB_BUILD_OPTIONS=nocheck
    dpkg-buildpackage -b -uc -us
  )

  mapfile -t local_debs < <(find /tmp/libvips-build -maxdepth 1 -type f -name '*.deb' | sort)
  if [ "${#local_debs[@]}" -eq 0 ]; then
    echo "failed to build libvips .deb packages" >&2
    exit 1
  fi

  log "Installing locally built libvips packages"
  apt-get install -y "${local_debs[@]}"
  vips --version
  pkg-config --modversion vips
}

build_and_test_nip2() {
  log "Building and testing nip2"
  apt-get build-dep -y nip2

  rm -rf /tmp/nip2-src
  mkdir -p /tmp/nip2-src
  (
    cd /tmp/nip2-src
    apt-get source nip2
  )

  local src_dir
  src_dir="$(find /tmp/nip2-src -maxdepth 1 -mindepth 1 -type d -name 'nip2-*' | head -n 1)"
  if [ -z "${src_dir}" ]; then
    echo "failed to locate nip2 source tree" >&2
    exit 1
  fi

  (
    cd "${src_dir}"
    ./configure --disable-silent-rules
    make -j"${JOBS}"
    chmod +x test/test_all.sh
    mkdir -p /tmp/nip2-home
    HOME=/tmp/nip2-home xvfb-run -a ./test/test_all.sh
  )
}

patch_photoqt_for_libvips_smoke_test() {
  local src_dir="$1"

  python3 - "${src_dir}" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])
cmake = root / "CMakeLists.txt"
list_files = root / "CMake" / "ListFilesCPlusPlus.cmake"
header = root / "testing" / "pqc_test.h"
cpp = root / "testing" / "pqc_test.cpp"

cmake_text = cmake.read_text()
old_test_link = (
    "    target_link_libraries(photoqt_test PRIVATE Qt6::Quick Qt6::Widgets "
    "Qt6::Sql Qt6::Core Qt6::Svg Qt6::Concurrent Qt6::Test)\n"
)
new_test_link = (
    "    target_link_libraries(photoqt_test PRIVATE Qt6::Quick Qt6::Widgets "
    "Qt6::Sql Qt6::Core Qt6::Svg Qt6::Concurrent Qt6::Multimedia "
    "Qt6::PrintSupport Qt6::DBus Qt6::Test)\n"
)
if old_test_link in cmake_text and new_test_link not in cmake_text:
    cmake.write_text(cmake_text.replace(old_test_link, new_test_link, 1))

list_files_text = list_files.read_text()
test_source_override = (
    "# Ensure the test executable uses the same implementation units as the app.\n"
    "SET(photoqt_testscripts_SOURCES ${photoqt_SOURCES})\n"
    "list(REMOVE_ITEM photoqt_testscripts_SOURCES cplusplus/main.cpp)\n"
    "SET(d \"testing\")\n"
    "SET(photoqt_testscripts_SOURCES ${photoqt_testscripts_SOURCES} ${d}/main.cpp ${d}/pqc_test.cpp ${d}/pqc_test.h)\n"
)
if "list(REMOVE_ITEM photoqt_testscripts_SOURCES cplusplus/main.cpp)" not in list_files_text:
    marker = (
        "SET(photoqt_testscripts_SOURCES ${photoqt_testscripts_SOURCES} "
        "${d}/pqc_scriptsimages.h ${d}/pqc_scriptsmetadata.h ${d}/pqc_scriptsother.h)\n"
        "SET(photoqt_testscripts_SOURCES ${photoqt_testscripts_SOURCES} "
        "${d}/pqc_scriptsshareimgur.h ${d}/pqc_scriptsshortcuts.h ${d}/pqc_scriptswallpaper.h)\n"
    )
    if marker not in list_files_text:
        raise SystemExit("photoqt test source list marker not found")
    list_files.write_text(list_files_text.replace(marker, marker + "\n" + test_source_override, 1))

header_text = header.read_text()
if "void testLibVipsBackend();" not in header_text:
    marker = "    void testListArchiveContentRar();\n    void testListArchiveContent7z();\n"
    replacement = marker + "\n    void testLibVipsBackend();\n"
    if marker not in header_text:
        raise SystemExit("photoqt test header marker not found")
    header.write_text(header_text.replace(marker, replacement, 1))

cpp_text = cpp.read_text()
include_marker = '#include <pqc_filefoldermodel.h>\n'
extra_includes = (
    '#include <pqc_filefoldermodel.h>\n'
    '#include <pqc_imageformats.h>\n'
    '#include <pqc_loadimage.h>\n'
    '#include <QtSql/QSqlQuery>\n'
)
if '#include <pqc_loadimage.h>\n' not in cpp_text:
    if include_marker not in cpp_text:
        raise SystemExit("photoqt test cpp include marker not found")
    cpp_text = cpp_text.replace(include_marker, extra_includes, 1)

if "void PQCTest::testLibVipsBackend()" not in cpp_text:
    marker = "void PQCTest::testListArchiveContentZip() {\n"
    snippet = """void PQCTest::testLibVipsBackend() {\n\n    const QString filename = QDir::tempPath()+\"/photoqt_test/libvips-backend.ppm\";\n\n    QImage source(8, 6, QImage::Format_RGB888);\n    source.fill(qRgb(0, 0, 255));\n    QVERIFY(source.save(filename, \"PPM\"));\n\n    PQCImageFormats::get();\n    QSqlDatabase db = QSqlDatabase::database(\"imageformats\");\n    QVERIFY(db.isOpen());\n\n    QSqlQuery query(db);\n    QVERIFY(query.exec(\n        \"UPDATE imageformats \"\n        \"SET enabled=1, qt=0, resvg=0, libvips=1, imagemagick=0, graphicsmagick=0, \"\n        \"libraw=0, poppler=0, xcftools=0, devil=0, freeimage=0, archive=0, video=0, libmpv=0 \"\n        \"WHERE endings LIKE '%ppm%'\"));\n\n    PQCImageFormats::get().readDatabase();\n    QVERIFY(PQCImageFormats::get().getEnabledFormatsLibVips().contains(\"ppm\"));\n    QVERIFY(!PQCImageFormats::get().getEnabledFormatsQt().contains(\"ppm\"));\n\n    QSize origSize(-1, -1);\n    QImage loaded;\n    const QString err = PQCLoadImage::get().load(filename, QSize(-1, -1), origSize, loaded);\n\n    QCOMPARE(err, QString(\"\"));\n    QCOMPARE(origSize, QSize(8, 6));\n    QVERIFY(!loaded.isNull());\n    QCOMPARE(loaded.size(), QSize(8, 6));\n\n}\n\n"""
    if marker not in cpp_text:
        raise SystemExit("photoqt test cpp insertion marker not found")
    cpp_text = cpp_text.replace(marker, snippet + marker, 1)

cpp.write_text(cpp_text)
PY
}

build_and_test_photoqt() {
  log "Building and testing photoqt"
  apt-get build-dep -y photoqt

  rm -rf /tmp/photoqt-src
  mkdir -p /tmp/photoqt-src
  (
    cd /tmp/photoqt-src
    apt-get source photoqt
  )

  local src_dir
  src_dir="$(find /tmp/photoqt-src -maxdepth 1 -mindepth 1 -type d -name 'photoqt-*' | head -n 1)"
  if [ -z "${src_dir}" ]; then
    echo "failed to locate photoqt source tree" >&2
    exit 1
  fi

  patch_photoqt_for_libvips_smoke_test "${src_dir}"

  (
    cd "${src_dir}"
    # Keep libvips as the only non-Qt image backend in this test build so the
    # smoke test cannot silently pass through an alternate loader fallback.
    cmake -S . -B build -G Ninja \
      -DCMAKE_BUILD_TYPE=Release \
      -DDEVIL=OFF \
      -DFREEIMAGE=OFF \
      -DGRAPHICSMAGICK=OFF \
      -DIMAGEMAGICK=OFF \
      -DLIBVIPS=ON \
      -DPOPPLER=OFF \
      -DRESVG=OFF \
      -DTESTING=ON
    cmake --build build --parallel "${JOBS}"
    mkdir -p /tmp/photoqt-home /tmp/photoqt-config
    HOME=/tmp/photoqt-home XDG_CONFIG_HOME=/tmp/photoqt-config xvfb-run -a ./build/photoqt_test testLibVipsBackend
  )
}

build_and_test_ruby_vips() {
  log "Building and testing ruby-vips"
  apt-get build-dep -y ruby-vips

  rm -rf /tmp/ruby-vips-src
  mkdir -p /tmp/ruby-vips-src
  (
    cd /tmp/ruby-vips-src
    apt-get source ruby-vips
  )

  local src_dir
  src_dir="$(find /tmp/ruby-vips-src -maxdepth 1 -mindepth 1 -type d -name 'ruby-vips-*' | head -n 1)"
  if [ -z "${src_dir}" ]; then
    echo "failed to locate ruby-vips source tree" >&2
    exit 1
  fi

  (
    cd "${src_dir}"
    rspec --backtrace -r ./spec/spec_helper.rb --pattern './spec/**/*_spec.rb'
  )
}

main() {
  enable_source_repositories
  install_base_tools
  load_dependents
  build_and_install_original_libvips

  local dep
  for dep in "${DEPENDENTS[@]}"; do
    case "${dep}" in
      nip2)
        build_and_test_nip2
        ;;
      photoqt)
        build_and_test_photoqt
        ;;
      ruby-vips)
        build_and_test_ruby_vips
        ;;
      *)
        echo "no test implementation for dependent: ${dep}" >&2
        exit 1
        ;;
    esac
  done

  log "All dependent builds and tests passed"
}

main "$@"
CONTAINER
