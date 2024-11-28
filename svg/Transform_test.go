// scale(0.25,0.25), translate(15,15)

package svg

import (
	"github.com/stretchr/testify/assert"
	"log"
	"testing"
)

func TestParseTransformList(t *testing.T) {
	var testCases = []struct{input string; expectErr bool; expectResult []Transform} {
		{"scale(0.25,0.26), translate(15,16)", false, []Transform{
			Transform{Name:"scale", Parameters: []float64{0.25,0.26}},
			Transform{Name:"translate", Parameters: []float64{15,16}}}},
		{"scale(0.25,0.26), translate(15,16), --", true, []Transform{}},
		{"scale(0.25,---)", true, []Transform{}},
		{"", false, []Transform(nil)},
	}

	for _, testCase := range testCases {
		result, err := ParseTransformList(testCase.input)

		pass := false
		if testCase.expectErr {
			pass = err != nil
		} else {
			pass = err == nil && assert.ObjectsAreEqual(testCase.expectResult,result)
		}

		log.Printf("[INFO] pass: %t input: '%s' expect: %v expectErr: %t err: %s\n",
			pass,
			testCase.input,
			testCase.expectResult,
			testCase.expectErr,
			err,
		)
		
		if testCase.expectErr {
			assert.Error(t, err)
		} else {
			assert.Nil(t, err)
			assert.Equal(t, testCase.expectResult, result)
		}
	}
}

func TestTransformApply(t *testing.T) {
	var testCases = []struct{inputTx Transform; x,y,rX,rY, delta float64} {
		{Transform{Name:"scale",     Parameters: []float64{2.0,3.0}},  4.0,5.0, 8.0,15.0, 0},
		{Transform{Name:"scale",     Parameters: []float64{2.0}},      4.0,5.0, 8.0,10.0, 0},
		{Transform{Name:"scale",     Parameters: []float64{}},         4.0,5.0, 4.0,5.0,  0},
		{Transform{Name:"translate", Parameters: []float64{2.0,3.0}},  4.0,5.0, 6.0,8.0,  0},
		{Transform{Name:"translate", Parameters: []float64{2.0}},      4.0,5.0, 6.0,5.0,  0},
		{Transform{Name:"translate", Parameters: []float64{}},         4.0,5.0, 4.0,5.0,  0},
		{Transform{Name:"rotate",    Parameters: []float64{90.0}},     4.0,0.0, 0.0,4.0,  1e-10},
		{Transform{Name:"rotate",    Parameters: []float64{90.0,2,0}}, 4.0,0.0, 2.0,2.0,  0},
		{Transform{Name:"skewX",     Parameters: []float64{30}},       4.0,4.0, 7.46,4.0, 0.1},
		{Transform{Name:"skewY",     Parameters: []float64{30}},          4.0, 0.0, 4.0,  3.46, 0.1},
		{Transform{Name:"matrix",    Parameters: []float64{1,0,0,1,0,0}}, 4.0, 5.0, 4.0,  5.0,  0.0}, // identity matrix
		{Transform{Name:"matrix",    Parameters: []float64{2,0,0,3,0,0}}, 4.0, 5.0, 8.0, 15.0,  0.0}, // scale, 2x and 3y
		{Transform{Name:"matrix",    Parameters: []float64{1,0,0,1,1,2}}, 4.0, 5.0, 5.0,  7.0,  0.0}, // translate by x+1, y+2
		{Transform{Name:"matrix",    Parameters: []float64{}},            4.0, 5.0, 4.0,  5.0,  0.0},
		{Transform{Name:"rotate",    Parameters: []float64{}},         4,5,4,5,0},
		{Transform{Name:"skewX",     Parameters: []float64{}},         4,5,4,5,0},
		{Transform{Name:"skewY",     Parameters: []float64{}},         4,5,4,5,0},
		{Transform{Name:"unknown",   Parameters: []float64{}},         4,5,4,5,0},
	}

	for _, testCase := range testCases {
		rX, rY := testCase.inputTx.Apply(testCase.x, testCase.y)

		log.Printf("[INFO] %s(%v) %v,%v => %v, %v (expected: %v, %v +- %v)\n", testCase.inputTx.Name,
			testCase.inputTx.Parameters, testCase.x, testCase.y, rX, rY, testCase.rX, testCase.rY, testCase.delta)

		assert.InDelta(t, testCase.rX, rX, testCase.delta)
		assert.InDelta(t, testCase.rY, rY, testCase.delta)
	}
}
