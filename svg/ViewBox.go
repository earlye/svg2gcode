package svg

import (
	"github.com/go-errors/errors"
	"regexp"
	"strconv"
	"svg2gcode/util"
)

type ViewBox struct {
	MinX float64
	MinY float64
	Width float64
	Height float64
}

var ViewBoxPattern = NumberPattern +
		",?[[:space:]]+" + NumberPattern +
		",?[[:space:]]+" + NumberPattern +
		",?[[:space:]]+" + NumberPattern
var ViewBoxRegexp = regexp.MustCompile("^" + ViewBoxPattern + "$")
func ParseViewBox(input string) (result ViewBox, err error) {
	match := ViewBoxRegexp.FindStringSubmatch(input)
	if (len(match) == 0) {
		err = errors.Errorf("input '%s' doesn't match ViewBoxRegexp", input)
		return
	}
	MinX, err := strconv.ParseFloat(match[1], 64);
	util.PanicOnError(err)

	MinY, err := strconv.ParseFloat(match[7], 64)
	util.PanicOnError(err)

	Width, err := strconv.ParseFloat(match[13], 64)
	util.PanicOnError(err)

	Height, err := strconv.ParseFloat(match[19], 64)
	util.PanicOnError(err)

	result = ViewBox{
		MinX: MinX,
		MinY: MinY,
		Width: Width,
		Height: Height,
	}
	return
}
