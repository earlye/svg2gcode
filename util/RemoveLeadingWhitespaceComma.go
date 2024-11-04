package util

import (
	"strings"
)


func RemoveLeadingWhitespaceComma(input string) (result string) {
	result = input
	for {
		next := strings.TrimSpace(result)
		next = strings.TrimPrefix(next, ",")
		if next == result {
			return
		}
		result = next
	}
}
