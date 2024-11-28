package util

import (
	"github.com/stretchr/testify/assert"
	_ "log"
	"testing"
)

func TestDetectName(t *testing.T) {
	name := DetectName()
	assert.NotEqual(t, "", name)
}
