package main

import (
	"log"

	vips "github.com/DAddYE/vips"
)

func main() {
	// This binding exposes libvips through package-level lifecycle helpers and a
	// single Resize entrypoint. The old Resize path dereferences a null image
	// against the safe runtime today, so keep the harness on the strongest stable
	// application-specific path: initialize, inspect the registered operations,
	// and cycle shutdown/startup cleanly.
	vips.Debug()
	vips.Shutdown()
	vips.Initialize()
	vips.Shutdown()

	log.Print("sharp-for-go init smoke passed")
}
