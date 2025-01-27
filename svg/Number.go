package svg

import (
	"fmt"
	"github.com/go-errors/errors"
	"log"
	"regexp"
	"strconv"
)

const NumberPattern = "(" +
		"[+-]?" +
		"(" +
		"([[:digit:]]+(\\.[[:digit:]]+)?)"+
		"|" +
		"(\\.[[:digit:]]+)"+
		")" +
		"([eE][+-]?[[:digit:]]+)?" +
		")"
var NumberRegexp = regexp.MustCompile("^" + NumberPattern + "$")
func ParseNumber(input string) (result float64, err error) {
	match := NumberRegexp.FindStringSubmatch(input)
	if (len(match) == 0) {
		err = errors.Errorf("input '%s' doesn't match NumberRegexp", input)
		return
	}
	result, err = strconv.ParseFloat(match[0], 64)
	return
}

func MustParseNumber(input string) (result float64) {
	result, err := ParseNumber(input)
	if err != nil {
		panic(err)
	}
	return
}

var PopNumberRegexp = regexp.MustCompile("^([[:space:]]*(" + NumberPattern + ")).*")
func PopNumber(input string) (result float64, remaining string, err error) {
	match := PopNumberRegexp.FindStringSubmatch(input)
	numberPart := ""
	index := 0
	if len(match) >= 2 {
		numberPart = match[2]
		index = len(match[1])
	}
	remaining = input[index:]
	// log.Printf("[WARN] input: '%s' match: '%#v' remain: '%s'", input, match, remaining)
	if (len(numberPart) == 0) {
		// log.Printf("[WARN] input '%s' doesn't start with whitespace and a number", input)
		err = errors.Errorf("input '%s' doesn't start with whitespace and a number", input)
		return
	}
	if len(remaining) != 0 && (remaining[0] == '.' || remaining[0] == 'e' || remaining[0] == 'E') {
		msg := fmt.Sprintf("[WARN] input '%s' starts with whitespace and a number, but continues with parts of a number that require additional digits", input)
		log.Println(msg)
		err = errors.Errorf("%s",msg)
		return
	}
	result, err = strconv.ParseFloat(numberPart, 64)
	return
}
