package util

import (
	"os"
	"path/filepath"
)

func DetectName() string {
	executable, err := os.Executable()
	PanicOnError(err)
	return filepath.Base(executable)
}
