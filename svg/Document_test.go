package svg

import (
	"github.com/stretchr/testify/assert"
	"log"
	"testing"
)

func TestDocumentAttributes(t *testing.T) {
	log.Printf("TestDocument...")
	val := Document{};
	assert.Equal(t, "", val.Height())
	assert.Equal(t, "", val.Width())
	assert.Equal(t, "", val.ViewBox())
}

