package svg

import (
	"github.com/stretchr/testify/assert"
	_ "log"
	"testing"
)

func TestParseNumber(t *testing.T) {
	var testCases = []struct{input string; expectVal float64; expectErr bool} {
		{"+", 0, true},
		{"-", 0, true},
		{"", 0, true},
		{"32.", 0, true},
		{"1e", 0, true},
		{"+32", float64(+32.0), false},			
		{"-32", float64(-32.0), false},			
		{"32", 32, false},
		{"32.5", 32.5, false},
		{".5", 0.5, false},
		{"-.5", -0.5, false},
		{"1e2", 100, false},
	}

	for _, testCase := range testCases {
		result, err := ParseNumber(testCase.input)

		if testCase.expectErr {
			// log.Printf("[INFO] pass: %t input: '%s' expect: %f expectErr: %t err: %s\n",
			// 	err != nil,
			// 	testCase.input,
			// 	testCase.expectVal,
			// 	testCase.expectErr,
			// 	err,
			// )
			assert.NotNil(t, err)
		} else {
			// log.Printf("[INFO] pass: %t input: '%s' expect: %f expectErr: %t err: %s\n",
			// 	err == nil && testCase.expectVal == result,
			// 	testCase.input,
			// 	testCase.expectVal,
			// 	testCase.expectErr,
			// 	err,
			// )
			assert.Nil(t, err)
			assert.Equal(t, testCase.expectVal, result)
		}
	}
}

func TestPopNumber(t *testing.T) {
	var testCases = []struct{
		input string;
		expectVal float64;
		expectRemain string;
		expectErr bool} {
		{"+", 0, "", true},
		{"-", 0, "", true},
		{"", 0, "", true},
		{"32.", 0, "", true},
		{"32).", 32, ").", false},
		{"1e", 0, "", true},
		{"+32", float64(+32.0), "", false},			
		{"-32", float64(-32.0), "", false},			
		{"32", 32, "", false},
		{"32.5", 32.5, "", false},
		{".5", 0.5, "", false},
		{"-.5", -0.5, "", false},
		{"1e2", 100, "", false},
		{"32 64", 32, " 64", false},
		{" 64", 64, "", false},		
		{"32,64", 32, ",64", false},
		{"64", 64, "", false},		
	}
	for _, testCase := range testCases {
		result, remaining, err := PopNumber(testCase.input)

		if testCase.expectErr {
			// log.Printf("[INFO] pass: %t input: '%s' expect: %f expectErr: %t err: %s\n",
			// 	err != nil,
			// 	testCase.input,
			// 	testCase.expectVal,
			// 	testCase.expectErr,
			// 	err,
			// )
			assert.NotNil(t, err)
		} else {
			// log.Printf("[INFO] pass: %t input: '%s' %f (actual %f)  remain '%s' (actual '%s') expectErr: %t err: %s\n",
			// 	err == nil && testCase.expectVal == result && testCase.expectRemain == remaining,
			// 	testCase.input,
			// 	testCase.expectVal,
			// 	result,
			// 	testCase.expectRemain,
			// 	remaining,
			// 	testCase.expectErr,
			// 	err,
			// )
			assert.Equal(t, testCase.expectVal, result)
			assert.Equal(t, testCase.expectRemain, remaining)
			assert.Nil(t, err)
		}
	}
}
