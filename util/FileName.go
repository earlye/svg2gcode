package util

import (
	"os"
)

func FileName[T interface{}](value T, defaultValue string) (result string) {
	filePtr, ok := any(value).(*os.File)
	if ok {
		result = filePtr.Name()
		return
	}
	file, ok := any(value).(os.File)
	if ok {
		result = file.Name()
		return
	}
	result = defaultValue
	return
}
