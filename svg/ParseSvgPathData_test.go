package svg

import (
	"github.com/stretchr/testify/assert"
	"log"
	"testing"
)

func TestParseSvgPathData(t *testing.T) {
	var testCases = []struct{input string; expectVal []PathCommand} {
		{"M 0,1 A 2,3", []PathCommand{
			PathCommand{"M",[]float64{0,1}},
			PathCommand{"A",[]float64{2,3}},
		}},
		{"+", []PathCommand(nil)},
		{"M1,2", []PathCommand{PathCommand{"M",[]float64{1,2}}}},
		{"M 0,0 25.4,25.4", []PathCommand{PathCommand{"M",[]float64{0,0,25.4,25.4}}}},
	}

	for _, testCase := range testCases {
		result := ParseSvgPathData(testCase.input)

		if !assert.ObjectsAreEqual(testCase.expectVal, result) {
			log.Printf("[ERROR] input: '%v' expect: '%#v' result:'%#v'\n",
				testCase.input,
				testCase.expectVal,
				result,
			)
			assert.Fail(t,"")
			return
		}
		assert.Equal(t, testCase.expectVal, result)
	}
	// assert.Fail(t,"")
}
