package svg

import (
	"github.com/go-errors/errors"
	"log"
	"math"
	"regexp"
	"svg2gcode/util"
	"strconv"
)

type Transform struct {
	Name string
	Parameters []float64
}

func matrix(x,y,a,b,c,d,e,f float64) (tx, ty float64) {
	// x   [ a c e
	// y     b d f
	//       0 0 1 ]
	tx = a * x + c * y + e
	ty = b * x + d * y + f
	return
}

func translate(x,y,dx,dy float64) (tx,ty float64) {
	tx = x + dx
	ty = y + dy
	return
}

func scale(x,y,sx,sy float64) (tx,ty float64) {
	tx = x * sx
	ty = y * sy
	return
}

func radians(deg float64) (r float64) {
	r = deg * (math.Pi/float64(180.0))
	return
}

func rotate(x,y,cx,cy,deg float64) (tx, ty float64) {
	a := radians(deg)
	tx,ty = matrix(x,y,
		math.Cos(a), math.Sin(a), -math.Sin(a),
		math.Cos(a),
		cx * (1 - math.Cos(a)) + cy * (math.Sin(a)),
		cy * (1 - math.Cos(a)) - cx * (math.Sin(a)))
	log.Printf("[INFO] rotate: %v,%v @%v[%v,%v] => %v,%v", x,y,deg,cx,cy,tx,ty)
	return
}

func skewX(x,y,deg float64) (tx,ty float64) {
	a := radians(deg)
	tx = x + math.Cos(a) * y
	ty = y
	return
}

func skewY(x,y,deg float64) (tx,ty float64) {
	a := radians(deg)
	tx = x
	ty = y + math.Cos(a) * x
	return
}


func (this Transform) Apply(x, y float64) (tx, ty float64) {
	tx = x
	ty = y
	switch(this.Name) {
	case "matrix":
		if len(this.Parameters) < 6 {
			log.Printf("[WARN] Not enough parameters for matrix")
			return
		}
		return matrix(x,y, this.Parameters[0], this.Parameters[1], this.Parameters[2], this.Parameters[3], this.Parameters[4], this.Parameters[5])
	case "translate":
		dx := float64(0)
		dy := float64(0)
		if len(this.Parameters) < 1 {
			log.Printf("[WARN] Not enough parameters for translate")
			return
		}
		dx = this.Parameters[0]
		if len(this.Parameters) >= 2 {
			dy = this.Parameters[1]
		}
		return translate(x,y,dx,dy)
	case "scale":
		sx := float64(1)
		sy := float64(1)
		if len(this.Parameters) < 1 {
			log.Printf("[WARN] Not enough parameters for scale")
			return
		}
		sx = this.Parameters[0]
		if len(this.Parameters) >= 2 {
			sy = this.Parameters[1]
		} else {
			sy = sx
		}
		return scale(x,y,sx,sy)
	case "rotate":
		deg := float64(0)
		cx := float64(0)
		cy := float64(0)

		if len(this.Parameters) < 1 {
			log.Printf("[WARN] Not enough parameters for rotate")
			return
		}

		deg = this.Parameters[0]

		if len(this.Parameters) >= 3 {
			cx = this.Parameters[1]
			cy = this.Parameters[2]
		}

		return rotate(x,y,cx,cy,deg)
	case "skewX":
		if len(this.Parameters) < 1 {
			log.Printf("[WARN] Not enough parameters for skewX")
			return
		}

		deg := this.Parameters[0]
		return skewX(x,y, deg)

	case "skewY":
		if len(this.Parameters) < 1 {
			log.Printf("[WARN] Not enough parameters for skewY")
			return
		}

		deg := this.Parameters[0]
		return skewY(x,y, deg)

	default:
		log.Printf("[WARN] Unsupported transform '%s'\n", this.Name)
		return
	}
}

const TransformPattern = "(([[:alpha:]][[:alnum:]]*)\\(([^\\)]*)\\))"
var TransformRegex = regexp.MustCompile(TransformPattern)
func ParseTransformList(input string) (result []Transform, err error) {
	remaining := input
	for {
		match := TransformRegex.FindStringSubmatch(remaining)
		if len(match) == 0 {
			if len(remaining) != 0 {
				err = errors.Errorf("Failed to consume all of transform list.")
			}
			return
		}

		log.Printf("[INFO] ParseTransformList: remaining: '%s' match: '%#v'\n", remaining, match)
		remaining = util.RemoveWhitespaceLeadingComma(remaining[len(match[0]):])
		name := match[2]
		parametersString := match[3]

		var parameters []float64
		parameters, err = ParseTransformParameters(parametersString)
		if err != nil {
			return
		}

		entry := Transform {Name: name, Parameters: parameters}
		result = append(result, entry)
	}
}

var TransformParameterRegex = regexp.MustCompile(NumberPattern)
func ParseTransformParameters(input string) (result []float64, err error) {
	remaining := input
	for {
		match := TransformParameterRegex.FindStringSubmatch(remaining)
		if len(match) == 0 {
			if len(remaining) != 0 {
				err = errors.Errorf("Failed to consume all of transform parameters list.")
			}
			return
		}

		var value float64
		value, err = strconv.ParseFloat(match[1], 64)
		util.PanicOnError(err)
		remaining = util.RemoveWhitespaceLeadingComma(remaining[len(match[0]):])
		result = append(result, value)
	}

}
