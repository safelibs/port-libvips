#!/usr/bin/env bash

run_case() {
  log "Testing govips"

  local src_dir=/tmp/govips-smoke
  rm -rf "${src_dir}"
  mkdir -p "${src_dir}"
  register_cleanup "${src_dir}"

  cat >"${src_dir}/go.mod" <<'EOF'
module safe-govips-smoke

go 1.22

require github.com/davidbyttow/govips/v2 v2.9.0
EOF

  cat >"${src_dir}/safe-vips-smoke.go" <<'EOF'
package main

import (
	"image"
	"image/color"
	"image/jpeg"
	"os"

	"github.com/davidbyttow/govips/v2/vips"
)

func main() {
	out, err := os.Create("input.jpg")
	if err != nil {
		panic(err)
	}
	img := image.NewRGBA(image.Rect(0, 0, 8, 6))
	for y := 0; y < 6; y++ {
		for x := 0; x < 8; x++ {
			img.Set(x, y, color.RGBA{R: 10, G: 20, B: 30, A: 255})
		}
	}
	if err := jpeg.Encode(out, img, nil); err != nil {
		panic(err)
	}
	if err := out.Close(); err != nil {
		panic(err)
	}

	vips.Startup(nil)
	defer vips.Shutdown()

	image1, err := vips.NewImageFromFile("input.jpg")
	if err != nil {
		panic(err)
	}
	defer image1.Close()

	if image1.Width() != 8 || image1.Height() != 6 {
		panic("govips loaded unexpected input dimensions")
	}

	metadata := image1.Metadata()
	if metadata.Width != 8 || metadata.Height != 6 {
		panic("govips metadata dimensions do not match the loaded image")
	}

	image2, err := vips.Black(4, 3)
	if err != nil {
		panic(err)
	}
	defer image2.Close()

	if image2.Width() != 4 || image2.Height() != 3 {
		panic("govips black image dimensions mismatch")
	}
}
EOF

  (
    cd "${src_dir}"
    go mod tidy
    run_manifest_smoke_command govips "${src_dir}"
  )
}
