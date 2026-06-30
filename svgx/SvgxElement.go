package svgx

import (
	"fmt"
	"log"
	"math"
	"strings"
	"svg2gcode/logx"
	"svg2gcode/svg"
)

type SvgxElement struct {
	GCodeDesc     *GCodeDesc
	EffectiveDesc *GCodeDesc `yaml:"omit"`
	Document      *SvgxDocument  `yaml:"omit"`
	Parent        *SvgxElement   `yaml:"omit"`
	XmlElement    *svg.XmlElement `yaml:"omit"`
	Children      []*SvgxElement
}

func (this *SvgxElement) MarshalYAML() (result interface{}, err error) {
	result = map[string]interface{}{
		"GCodeDesc":  this.GCodeDesc,
		"XmlElement": this.XmlElement,
	}
	return
}

// --- pure helpers ---

func useAbsoluteLines(ctx CarveCtx) (lines []string, out CarveCtx) {
	out = ctx
	if out.UsingAbsolute {
		return
	}
	lines = append(lines, "G90 ; Use absolute positioning.\n")
	out.UsingAbsolute = true
	return
}

func liftToSafeHeightLines(ctx CarveCtx) (lines []string, out CarveCtx) {
	out = ctx
	if out.Z == out.SafeHeight {
		return
	}
	tx, ty := svg.ApplyTransformList(out.X, out.Y, out.Transforms)
	tx *= out.MmPerUnit
	ty *= out.MmPerUnit
	lines = append(lines, fmt.Sprintf("G0 F1000 X%f Y%f Z%f; (lift tosafe-height)\n", tx, ty, out.SafeHeight))
	out.Z = out.SafeHeight
	return
}

func lineAbsoluteLines(x, y float64, ctx CarveCtx) (lines []string, out CarveCtx) {
	out = ctx
	var absLines []string
	absLines, out = useAbsoluteLines(out)
	lines = append(lines, absLines...)
	out.X, out.Y = x, y
	tx, ty := svg.ApplyTransformList(x, y, out.Transforms)
	tx *= out.MmPerUnit
	ty *= out.MmPerUnit
	lines = append(lines, fmt.Sprintf("G1 F1000 X%f Y%f Z%f; (line-absolute: %f,%f)\n",
		tx, ty, out.Depth, x, y))
	out.Z = out.Depth
	return
}

// --- path handlers (pure functions, Candidates 2 & 3) ---

func LineToAbsolute(params []float64, ctx CarveCtx) (lines []string, out CarveCtx) {
	defer logx.Indent(2)()
	out = ctx
	for len(params) >= 2 {
		var l []string
		l, out = lineAbsoluteLines(params[0], params[1], out)
		lines = append(lines, l...)
		params = params[2:]
	}
	return
}

func MoveAbsolute(params []float64, ctx CarveCtx) (lines []string, out CarveCtx) {
	defer logx.Indent(2)()
	out = ctx
	if len(params) < 2 {
		return
	}
	x, y := params[0], params[1]
	var l []string
	l, out = useAbsoluteLines(out)
	lines = append(lines, l...)
	l, out = liftToSafeHeightLines(out)
	lines = append(lines, l...)
	out.X, out.Y = x, y
	out.StartX, out.StartY = x, y
	tx, ty := svg.ApplyTransformList(x, y, out.Transforms)
	tx *= out.MmPerUnit
	ty *= out.MmPerUnit
	lines = append(lines,
		fmt.Sprintf("G0 F1000 X%f Y%f Z%f; (move-absolute: %f[%f],%f[%f] - safe-height)\n",
			tx, ty, out.SafeHeight, out.X, x, out.Y, y),
		fmt.Sprintf("G0 F1000 X%f Y%f Z%f; (move-absolute: %f[%f],%f[%f])\n",
			tx, ty, out.Depth, out.X, x, out.Y, y),
	)
	out.Z = out.Depth
	l, out = LineToAbsolute(params[2:], out)
	lines = append(lines, l...)
	return
}

func LineToRelative(params []float64, ctx CarveCtx) (lines []string, out CarveCtx) {
	defer logx.Indent(2)()
	out = ctx
	for len(params) >= 2 {
		var l []string
		l, out = lineAbsoluteLines(out.X+params[0], out.Y+params[1], out)
		lines = append(lines, l...)
		params = params[2:]
	}
	return
}

func MoveRelative(params []float64, ctx CarveCtx) (lines []string, out CarveCtx) {
	defer logx.Indent(2)()
	out = ctx
	if len(params) < 2 {
		return
	}
	dx, dy := params[0], params[1]
	var l []string
	l, out = useAbsoluteLines(out)
	lines = append(lines, l...)
	l, out = liftToSafeHeightLines(out)
	lines = append(lines, l...)
	out.X = out.X + dx
	out.Y = out.Y + dy
	out.StartX = out.X
	out.StartY = out.Y
	tx, ty := svg.ApplyTransformList(out.X, out.Y, out.Transforms)
	tx *= out.MmPerUnit
	ty *= out.MmPerUnit
	lines = append(lines,
		fmt.Sprintf("G0 F1000 X%f Y%f Z%f; (move-relative: %f,%f - safe-height)\n",
			tx, ty, out.SafeHeight, dx, dy),
		fmt.Sprintf("G0 F1000 X%f Y%f Z%f; (move-relative: %f,%f)\n",
			tx, ty, out.Depth, dx, dy),
	)
	out.Z = out.Depth
	l, out = LineToRelative(params[2:], out)
	lines = append(lines, l...)
	return
}

func DegreesToRadians(input float64) float64 {
	return input / 180 * math.Pi
}

func AngleRadians(uX, uY, vX, vY float64) float64 {
	// Eq 5.4 https://www.w3.org/TR/SVG/implnote.html#ArcImplementationNotes
	dotProduct := uX*vX + uY*vY
	magU := math.Sqrt(uX*uX + uY*uY)
	magV := math.Sqrt(uX*uX + uY*uY)

	sign := uX*vY - uY*vX
	if sign < 0 {
		sign = -1
	} else {
		sign = 1
	}

	return sign * math.Acos(dotProduct/(magU*magV))
}

func EllipticArcAbsolute(params []float64, ctx CarveCtx) (lines []string, out CarveCtx) {
	defer logx.Indent(2)()
	out = ctx

	for len(params) >= 7 {
		x1 := out.X
		y1 := out.Y
		rx := params[0]
		ry := params[1]
		xAxisRotation := params[2]
		largeArcFlag := params[3]
		sweepFlag := params[4]
		x2 := params[5]
		y2 := params[6]

		phi := DegreesToRadians(xAxisRotation)
		fA := largeArcFlag
		fS := sweepFlag

		// Eq 5.1
		u := (x1 - x2) / 2
		v := (y1 - y2) / 2
		x1Prime := math.Cos(phi)*u - math.Sin(phi)*v
		y1Prime := math.Sin(phi)*u + math.Cos(phi)*v

		// Eq 5.2
		coeff := math.Sqrt((rx*rx*ry*ry - rx*rx*y1Prime*y1Prime - ry*ry*x1Prime*x1Prime) / (rx*rx*y1Prime*y1Prime + ry*ry*x1Prime*x1Prime))
		if fA == fS {
			coeff = -coeff
		}
		cxPrime := coeff * (rx * y1Prime / ry)
		cyPrime := coeff * (-ry * x1Prime / rx)

		// Eq 5.3
		u = (x1 + x2) / 2
		v = (y1 + y2) / 2
		cx := math.Cos(phi)*cxPrime - math.Sin(phi)*cyPrime + (x1+x2)/2
		cy := math.Sin(phi)*cxPrime + math.Cos(phi)*cyPrime + (y1+y2)/2

		// Eq 5.5
		v1 := (x1Prime - cxPrime) / rx
		v2 := (y1Prime - cyPrime) / ry
		theta1 := AngleRadians(1, 0, v1, v2)
		// Eq 5.6
		deltaTheta := AngleRadians(v1, v2, (-x1Prime-cxPrime)/rx, (-y1Prime-cyPrime)/ry)
		for deltaTheta > 2.0*math.Pi {
			deltaTheta = deltaTheta - 2.0*math.Pi
		}
		for deltaTheta < -(2.0 * math.Pi) {
			deltaTheta = deltaTheta + 2.0*math.Pi
		}
		if sweepFlag == 0 && deltaTheta > 0 {
			deltaTheta = deltaTheta - 2.0*math.Pi
		}
		if sweepFlag == 1 && deltaTheta < 0 {
			deltaTheta = deltaTheta + 2.0*math.Pi
		}

		steps := int(math.Ceil(math.Abs(deltaTheta) / DegreesToRadians(10.0)))
		if steps < 1 {
			steps = 1
		}
		var l []string
		for i := 0; i < steps; i++ {
			theta := theta1 + float64(i)/float64(steps)*deltaTheta
			u = rx * math.Cos(theta)
			v = ry * math.Sin(theta)
			x := math.Cos(phi)*u - math.Sin(phi)*v + cx
			y := math.Sin(phi)*u + math.Cos(phi)*v + cy
			l, out = lineAbsoluteLines(x, y, out)
			lines = append(lines, l...)
		}
		l, out = lineAbsoluteLines(x2, y2, out)
		lines = append(lines, l...)
		params = params[7:]
	}
	return
}

func EllipticArcRelative(params []float64, ctx CarveCtx) (lines []string, out CarveCtx) {
	defer logx.Indent(2)()
	out = ctx

	for len(params) >= 7 {
		rx := params[0]
		ry := params[1]
		xAxisRotation := params[2]
		largeArcFlag := params[3]
		sweepFlag := params[4]
		x := out.X + params[5]
		y := out.Y + params[6]

		var l []string
		l, out = EllipticArcAbsolute([]float64{rx, ry, xAxisRotation, largeArcFlag, sweepFlag, x, y}, out)
		lines = append(lines, l...)
		params = params[7:]
	}
	return
}

func Close(params []float64, ctx CarveCtx) (lines []string, out CarveCtx) {
	return LineToAbsolute([]float64{ctx.StartX, ctx.StartY}, ctx)
}

func Interpolate(start, end, t float64) float64 {
	return start + t*(end-start)
}

func CubicBezierCurveAbsolute(params []float64, ctx CarveCtx) (lines []string, out CarveCtx) {
	defer logx.Indent(2)()
	out = ctx

	for len(params) >= 6 {
		x0, y0 := out.X, out.Y
		x1 := params[0]; y1 := params[1]
		x2 := params[2]; y2 := params[3]
		x3 := params[4]; y3 := params[5]

		for t := 0.0; t < 1.0; t = t + 0.1 {
			xA := Interpolate(x0, x1, t); yA := Interpolate(y0, y1, t)
			xB := Interpolate(x1, x2, t); yB := Interpolate(y1, y2, t)
			xC := Interpolate(x2, x3, t); yC := Interpolate(y2, y3, t)
			xAB := Interpolate(xA, xB, t); yAB := Interpolate(yA, yB, t)
			xBC := Interpolate(xB, xC, t); yBC := Interpolate(yB, yC, t)
			x := Interpolate(xAB, xBC, t); y := Interpolate(yAB, yBC, t)
			var l []string
			l, out = lineAbsoluteLines(x, y, out)
			lines = append(lines, l...)
		}
		var l []string
		l, out = lineAbsoluteLines(x3, y3, out)
		lines = append(lines, l...)
		params = params[6:]
	}
	return
}

func CubicBezierCurveRelative(params []float64, ctx CarveCtx) (lines []string, out CarveCtx) {
	defer logx.Indent(2)()
	out = ctx

	for len(params) >= 6 {
		x0, y0 := out.X, out.Y
		x1 := out.X + params[0]; y1 := out.Y + params[1]
		x2 := out.X + params[2]; y2 := out.Y + params[3]
		x3 := out.X + params[4]; y3 := out.Y + params[5]

		for t := 0.0; t < 1.0; t = t + 0.1 {
			xA := Interpolate(x0, x1, t); yA := Interpolate(y0, y1, t)
			xB := Interpolate(x1, x2, t); yB := Interpolate(y1, y2, t)
			xC := Interpolate(x2, x3, t); yC := Interpolate(y2, y3, t)
			xAB := Interpolate(xA, xB, t); yAB := Interpolate(yA, yB, t)
			xBC := Interpolate(xB, xC, t); yBC := Interpolate(yB, yC, t)
			x := Interpolate(xAB, xBC, t); y := Interpolate(yAB, yBC, t)
			var l []string
			l, out = lineAbsoluteLines(x, y, out)
			lines = append(lines, l...)
		}
		var l []string
		l, out = lineAbsoluteLines(x3, y3, out)
		lines = append(lines, l...)
		params = params[6:]
	}
	return
}

func CubicBezierSmoothCurveAbsolute(params []float64, ctx CarveCtx) (lines []string, out CarveCtx) {
	defer logx.Indent(2)()
	out = ctx

	for len(params) >= 4 {
		x1 := params[0]; y1 := params[1]
		x := params[2]; y := params[3]
		var l []string
		l, out = lineAbsoluteLines(x1, y1, out)
		lines = append(lines, l...)
		l, out = lineAbsoluteLines(x, y, out)
		lines = append(lines, l...)
		params = params[4:]
	}
	return
}

func CubicBezierSmoothCurveRelative(params []float64, ctx CarveCtx) (lines []string, out CarveCtx) {
	defer logx.Indent(2)()
	out = ctx

	for len(params) >= 4 {
		x0, y0 := out.X, out.Y
		dx1 := x0 + params[0]; dy1 := y0 + params[1]
		dx := x0 + params[2]; dy := y0 + params[3]
		var l []string
		l, out = lineAbsoluteLines(dx1, dy1, out)
		lines = append(lines, l...)
		l, out = lineAbsoluteLines(dx, dy, out)
		lines = append(lines, l...)
		params = params[4:]
	}
	return
}

func HorizontalAbsolute(params []float64, ctx CarveCtx) (lines []string, out CarveCtx) {
	defer logx.Indent(2)()
	out = ctx

	for len(params) >= 1 {
		x := params[0]
		y := out.Y
		var l []string
		l, out = lineAbsoluteLines(x, y, out)
		lines = append(lines, l...)
		params = params[1:]
	}
	return
}

func HorizontalRelative(params []float64, ctx CarveCtx) (lines []string, out CarveCtx) {
	defer logx.Indent(2)()
	out = ctx

	for len(params) >= 1 {
		x := out.X + params[0]
		y := out.Y
		var l []string
		l, out = lineAbsoluteLines(x, y, out)
		lines = append(lines, l...)
		params = params[1:]
	}
	return
}

func QuadraticBezierCurveAbsolute(params []float64, ctx CarveCtx) (lines []string, out CarveCtx) {
	defer logx.Indent(2)()
	out = ctx

	for len(params) >= 4 {
		x0, y0 := out.X, out.Y
		x1 := params[0]; y1 := params[1]
		x2 := params[2]; y2 := params[3]

		for t := 0.0; t < 1.0; t = t + 0.1 {
			xA := Interpolate(x0, x1, t); yA := Interpolate(y0, y1, t)
			xB := Interpolate(x1, x2, t); yB := Interpolate(y1, y2, t)
			x := Interpolate(xA, xB, t); y := Interpolate(yA, yB, t)
			var l []string
			l, out = lineAbsoluteLines(x, y, out)
			lines = append(lines, l...)
		}
		var l []string
		l, out = lineAbsoluteLines(x2, y2, out)
		lines = append(lines, l...)
		params = params[4:]
	}
	return
}

func QuadraticBezierCurveRelative(params []float64, ctx CarveCtx) (lines []string, out CarveCtx) {
	defer logx.Indent(2)()
	out = ctx

	for len(params) >= 4 {
		x0, y0 := out.X, out.Y
		x1 := out.X + params[0]; y1 := out.Y + params[1]
		x2 := out.X + params[2]; y2 := out.Y + params[3]

		for t := 0.0; t < 1.0; t = t + 0.1 {
			xA := Interpolate(x0, x1, t); yA := Interpolate(y0, y1, t)
			xB := Interpolate(x1, x2, t); yB := Interpolate(y1, y2, t)
			x := Interpolate(xA, xB, t); y := Interpolate(yA, yB, t)
			var l []string
			l, out = lineAbsoluteLines(x, y, out)
			lines = append(lines, l...)
		}
		var l []string
		l, out = lineAbsoluteLines(x2, y2, out)
		lines = append(lines, l...)
		params = params[4:]
	}
	return
}

func QuadraticBezierSmoothCurveAbsolute(params []float64, ctx CarveCtx) (lines []string, out CarveCtx) {
	defer logx.Indent(2)()
	out = ctx

	for len(params) >= 2 {
		x1 := params[0]; y1 := params[1]
		var l []string
		l, out = lineAbsoluteLines(x1, y1, out)
		lines = append(lines, l...)
		params = params[2:]
	}
	return
}

func QuadraticBezierSmoothCurveRelative(params []float64, ctx CarveCtx) (lines []string, out CarveCtx) {
	defer logx.Indent(2)()
	out = ctx

	for len(params) >= 2 {
		dx1 := out.X + params[0]; dy1 := out.Y + params[1]
		var l []string
		l, out = lineAbsoluteLines(dx1, dy1, out)
		lines = append(lines, l...)
		params = params[2:]
	}
	return
}

func VerticalAbsolute(params []float64, ctx CarveCtx) (lines []string, out CarveCtx) {
	defer logx.Indent(2)()
	out = ctx

	for len(params) >= 1 {
		x := out.X
		y := params[0]
		var l []string
		l, out = lineAbsoluteLines(x, y, out)
		lines = append(lines, l...)
		params = params[1:]
	}
	return
}

func VerticalRelative(params []float64, ctx CarveCtx) (lines []string, out CarveCtx) {
	defer logx.Indent(2)()
	out = ctx

	for len(params) >= 1 {
		x := out.X
		y := out.Y + params[0]
		var l []string
		l, out = lineAbsoluteLines(x, y, out)
		lines = append(lines, l...)
		params = params[1:]
	}
	return
}

// pathHandlers is the dispatch table (Candidate 3).
var pathHandlers = map[string]PathHandler{
	"A": EllipticArcAbsolute,
	"a": EllipticArcRelative,
	"C": CubicBezierCurveAbsolute,
	"c": CubicBezierCurveRelative,
	"H": HorizontalAbsolute,
	"h": HorizontalRelative,
	"L": LineToAbsolute,
	"l": LineToRelative,
	"M": MoveAbsolute,
	"m": MoveRelative,
	"Q": QuadraticBezierCurveAbsolute,
	"q": QuadraticBezierCurveRelative,
	"S": CubicBezierSmoothCurveAbsolute,
	"s": CubicBezierSmoothCurveRelative,
	"T": QuadraticBezierSmoothCurveAbsolute,
	"t": QuadraticBezierSmoothCurveRelative,
	"V": VerticalAbsolute,
	"v": VerticalRelative,
	"Z": Close,
	"z": Close,
}

func carveSvgCircle(input *svg.XmlElement, writer *GCodeWriter, transforms []svg.Transform) {
	defer logx.Indent(2)()

	cxStr := input.AttributeDefault("cx", "0")
	cyStr := input.AttributeDefault("cy", "0")
	rxStr := input.AttributeDefault("rx", "auto")
	ryStr := input.AttributeDefault("ry", "auto")

	if strings.HasSuffix(cxStr, "%") || strings.HasSuffix(cyStr, "%") ||
		strings.HasSuffix(rxStr, "%") || strings.HasSuffix(ryStr, "%") {
		log.Printf("ERROR: percentages not yet supported.")
		return
	}

	cx := svg.MustParseNumber(cxStr)
	cy := svg.MustParseNumber(cyStr)
	rx := svg.MustParseNumber(rxStr)
	ry := svg.MustParseNumber(ryStr)

	pathData := fmt.Sprintf("M %f %f A %f %f A %f %f A %f %f A %f %f",
		cx+rx, cy,
		cx, cy+ry,
		cx-rx, cy,
		cx, cy-ry,
		cx+rx, cy)
	carveSvgPathData(pathData, writer, transforms)
}

func carveSvgPath(input *svg.XmlElement, writer *GCodeWriter, transforms []svg.Transform) {
	defer logx.Indent(2)()

	pathData := input.Attribute("d")
	carveSvgPathData(pathData, writer, transforms)
}

func carveSvgPathData(pathData string, writer *GCodeWriter, transforms []svg.Transform) {
	writer.Comment(fmt.Sprintf("-- Carving path Data: %s\n", pathData))
	writer.SetTransforms(transforms)
	writer.Ctx.PathCursor = PathCursor{}
	writer.CommentCurrentXY("  Carving path data 0,0 is")

	pathCommands := svg.ParseSvgPathData(pathData)
	for _, command := range pathCommands {
		writer.Comment(fmt.Sprintf("[DEBUG] %#v\n", command))
		handler, ok := pathHandlers[command.Command]
		if !ok {
			log.Printf("[WARN] Unexpected command '%s'", command.Command)
			continue
		}
		lines, newCtx := handler(command.Parameters, writer.Ctx)
		writer.Ctx = newCtx
		for _, line := range lines {
			writer.Write(line)
		}
	}
	writer.LiftToSafeHeight()
}

func (this *SvgxElement) Carve(writer *GCodeWriter, transforms []svg.Transform) {
	elementTransformStr := this.XmlElement.Attribute("transform")
	if len(elementTransformStr) != 0 {
		elementTransform, _ := svg.ParseTransformList(elementTransformStr)
		transforms = append(elementTransform, transforms...)
	}

	switch this.XmlElement.Name {
	case "":
		return
	case "http://www.w3.org/2000/svg:desc":
		return
	default:
		writer.Comment(fmt.Sprintf("this.XmlElement.id: %s this.XmlElement.Name: %#v transform: %#v\n",
			this.XmlElement.Attribute("id"),
			this.XmlElement.Name,
			transforms))
	}

	// Candidate 4: use pre-computed EffectiveDesc instead of parent-chain walk
	gcodeDesc := this.EffectiveDesc

	if gcodeDesc != nil && gcodeDesc.CarveDepth != "" {
		writer.Ctx.SafeHeight = gcodeDesc.GetSafeHeight(10)
		writer.LiftToSafeHeight()
		writer.Ctx.Depth = 0
		carveDepth := gcodeDesc.GetCarveDepth(0)
		for writer.Ctx.Depth < carveDepth {
			writer.Ctx.Depth = writer.Ctx.Depth + 1
			if writer.Ctx.Depth > carveDepth {
				writer.Ctx.Depth = carveDepth
			}
			writer.Comment(fmt.Sprintf("-- CurrentDepth: %f CarveDepth: %f\n", writer.Ctx.Depth, carveDepth))
			switch this.XmlElement.Name {
			case "http://www.w3.org/2000/svg:path":
				carveSvgPath(this.XmlElement, writer, transforms)
			case "http://www.w3.org/2000/svg:circle":
				carveSvgCircle(this.XmlElement, writer, transforms)
			}
		}
	}

	defer logx.Indent(2)()
	for _, child := range this.Children {
		child.Carve(writer, transforms)
	}
}
