package svg

import (
	"log"
	"strings"
)

type PathCommand struct {
	Command string
	Parameters []float64
}

func TrimSpaceAndPrefixes(input string, prefixes []string) (result string) {
	result = input
	for {
		next := strings.TrimSpace(result)
		for _, prefix := range(prefixes) {
			next = strings.TrimPrefix(next, prefix)
		}
		if result == next {
			return
		}
		result = next
	}
}

func PopSvgPathParameters(input string) (result []float64, remaining string) {
	var err error
	remaining = input
	for {
		var parameter float64
		remaining = TrimSpaceAndPrefixes(remaining, []string{","})
		parameter, remaining, err = PopNumber(remaining)
		if err != nil {
			return
		}
		result = append(result, parameter)
	}
}

func ParseSvgPathData(input string) (result []PathCommand) {
	// log.Printf("[INFO] ParseSvgPathData(%s)", input)
	remaining := input
	for {
		// log.Printf("remaining: %s", remaining)
		remaining = strings.TrimSpace(remaining)
		if len(remaining) == 0 {
			return
		}

		command := string(remaining[0])
		remaining = remaining[1:]
		switch command {
		case "M", "m", "L", "l", "H", "h", "V", "v", "C", "c", "S", "s", "Q", "q", "T", "t", "A", "a", "Z", "z":
			var parameters []float64
			parameters, remaining = PopSvgPathParameters(remaining)
			entry := PathCommand {
				Command: command,
				Parameters: parameters,
			}
			result = append(result, entry)
			// log.Printf("[INFO] result: %#v remaining: %#v", result, remaining)
		default:
			log.Printf("[WARN] Unexpected command '%s'", command)
		}
	}
}
