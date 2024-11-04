package svg

import (
	"github.com/stretchr/testify/assert"
	"log"
	"testing"
)

func TestParseViewBox(t *testing.T) {
	var testCases = []struct{input string; expectErr bool; expectVal ViewBox} {
		{"+", true, ViewBox{} },
		{"-", true, ViewBox{} },
		{"",  true, ViewBox{} },
		{"0 0 25.4 25.4", false, ViewBox{ MinX:0, MinY: 0, Width: 25.4, Height: 25.4} },
		{"1 2 3 4", false, ViewBox{ MinX:1, MinY: 2, Width: 3, Height: 4} },
	}

	for _, testCase := range testCases {
		result, err := ParseViewBox(testCase.input)

		if testCase.expectErr {
			log.Printf("[INFO] pass: %t input: '%s' expect: %f expectErr: %t err: %s\n",
				err != nil,
				testCase.input,
				testCase.expectVal,
				testCase.expectErr,
				err,
			)
			assert.NotNil(t, err)
		} else {
			log.Printf("[INFO] pass: %t input: '%s' expect: %f expectErr: %t err: %s\n",
				err == nil && (testCase.expectVal == result),
				testCase.input,
				testCase.expectVal,
				testCase.expectErr,
				err,
			)
			assert.Nil(t, err)
			assert.Equal(t, testCase.expectVal, result)
		}
	}
}
