// -*- tab-width: 2 -*-
/// coverage-ignore
package main

import (
	"log"
	"os"
	"svg2gcode/cli"
	"svg2gcode/consts"
)

func main() {
	log.Printf("[INFO] svg2gcode %s", consts.Version)

	os.Exit(cli.Execute())
}
