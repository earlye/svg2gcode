package svg

import "strings"

// ParseLengthMm converts an SVG length string to millimeters.
// Unitless values and "mm" are returned as-is. Other units are converted.
func ParseLengthMm(s string) (mm float64, err error) {
	value, remaining, err := PopNumber(s)
	if err != nil {
		return
	}
	switch strings.TrimSpace(remaining) {
	case "cm":
		mm = value * 10
	case "in":
		mm = value * 25.4
	case "pt":
		mm = value * 25.4 / 72
	case "pc":
		mm = value * 25.4 / 6
	case "px":
		mm = value * 25.4 / 96
	default: // "mm" or unitless — treat as mm
		mm = value
	}
	return
}
