package util

import (
	"github.com/stretchr/testify/assert"
	"testing"
)

func TestRemoveWhitespaceLeadingComma(t *testing.T) {
	var testCases = []struct{ input string; expected string }{
		{" , 2", "2"},
		{",2", "2"},
		{" , 2 ", "2"},
		{"2","2"},
		{",, 2", ", 2"},
	}
	for _, testCase := range testCases {
		actual := RemoveWhitespaceLeadingComma(testCase.input)
		assert.Equal(t, testCase.expected, actual)
	}
}
