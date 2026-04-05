#!/usr/bin/env bash

run_case() {
  log "Testing bimg"

  local src_dir=/tmp/bimg-smoke
  rm -rf "${src_dir}"
  mkdir -p "${src_dir}"
  register_cleanup "${src_dir}"

  cat >"${src_dir}/go.mod" <<'EOF'
module safe-bimg-smoke

go 1.22

require github.com/h2non/bimg v1.1.9
EOF

  cat >"${src_dir}/safe-vips-smoke.go" <<'EOF'
package main

import (
	"log"

	"github.com/h2non/bimg"
)

func main() {
	if bimg.VipsVersion == "" {
		log.Fatal("bimg did not expose a libvips version")
	}
	if !bimg.VipsIsTypeSupported(bimg.JPEG) {
		log.Fatal("bimg did not detect JPEG support")
	}
	if !bimg.VipsIsTypeSupported(bimg.PNG) {
		log.Fatal("bimg did not detect PNG support")
	}
	if memory := bimg.VipsMemory(); memory.Memory < 0 || memory.Allocations < 0 {
		log.Fatalf("unexpected bimg memory stats: %+v", memory)
	}

	bimg.VipsDebugInfo()
	bimg.Shutdown()
	bimg.Initialize()
	bimg.Shutdown()

	log.Print("bimg init smoke passed")
}
EOF

  (
    cd "${src_dir}"
    go mod tidy
    run_manifest_smoke_command bimg "${src_dir}"
  )
}
