package svg

import (
	"encoding/xml"
	"github.com/stretchr/testify/assert"
	"log"
	"testing"
)

func TestParseSvgString(t *testing.T) {
	var testCases = []struct{ input string; expectErr bool }{
		{"</svg>", true},
		{"<svg foo='bar'></svg>", false},
		{"<?xml version='1.0' encoding='UTF-8'?>\n<!-- comment --><svg foo='bar'></svg>\n\n<?nope?><!-- comment -->", false},
		{"<svg><g>\n</g></svg>", false},
		{"<svg></bar></svg>", true},
		{"<foo:svg></foo:svg>", true},
		{"<foo></foo>", true},
		{"", true},
	}
	log.Printf("TestParseSvgString...")

	for _, testCase := range testCases {
		_, err := ParseSvgString(testCase.input)
		if testCase.expectErr {
			assert.Error(t, err, "input: " + testCase.input)
		} else {
			assert.Nil(t, err, "input: " + testCase.input)
		}
	}
}

func TestDecodeSvgDocument(t *testing.T) {
	brokenDecoderToken := func(decoder *xml.Decoder) (xml.Token, error) {
		return xml.Directive{}, nil
	}
	decoder := xml.Decoder{}
	_, err := DecodeSvgDocument(&decoder, brokenDecoderToken)
	assert.Error(t, err)
} 

func TestDecodeSvgBackmatter(t *testing.T) {
	brokenDecoderToken := func(decoder *xml.Decoder) (xml.Token, error) {
		return xml.Directive{}, nil
	}
	decoder := xml.Decoder{}
	document := Document{}
	err := DecodeSvgBackmatter(&document, &decoder, brokenDecoderToken)
	assert.Error(t, err)
} 
