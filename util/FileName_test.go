package util

import (
	"github.com/stretchr/testify/assert"
	"os"
	"testing"
)

func TestFileName(t *testing.T) {
	fileptr, err := os.Open("FileName_test.go")
	assert.Nil(t, err)
	assert.NotNil(t, fileptr)

	name := FileName(fileptr, "foo")
	assert.Equal(t, "FileName_test.go", name)

	name = FileName(*fileptr, "foo")
	assert.Equal(t, "FileName_test.go", name)

	name = FileName("not-a-file", "foo")
	assert.Equal(t, "FileName_test.go", name)
}
