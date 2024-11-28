package util

import (
	"strings"
)

func RemoveWhitespaceLeadingComma(input string) (result string) {
	result = input
	result = strings.TrimSpace(result)
	result = strings.TrimPrefix(result, ",")
	result = strings.TrimSpace(result)
	return
}
