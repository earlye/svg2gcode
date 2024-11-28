package util

import (
	_ "github.com/stretchr/testify/assert"
	_ "log"
	"testing"
	"fmt"
)

func TestPanicOnError(t *testing.T) {
	PanicOnError(nil)
	
	defer func() {
		if r := recover(); r == nil {
			t.Errorf("The code did not panic")
		}
	}()	
	PanicOnError(fmt.Errorf("yep"))
}
