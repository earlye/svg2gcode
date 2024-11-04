package util

import (
	"os"
	"path/filepath"
)

func DetectName() string {
	executable, error := os.Executable()
	if (error != nil) {
		return "couldn't detect name"
	}

	return filepath.Base(executable)
}
