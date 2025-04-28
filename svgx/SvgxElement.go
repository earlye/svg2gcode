package svgx

import (
	"fmt"
	"log"
	"math"
	"svg2gcode/logx"
	"svg2gcode/svg"
	"strings"
)

type SvgxElement struct {
	GCodeDesc *GCodeDesc
	Document *SvgxDocument `yaml:"omit"`
	Parent *SvgxElement `yaml:"omit"`
	XmlElement *svg.XmlElement `yaml:"omit"`
	Children []*SvgxElement
}

func (this *SvgxElement) MarshalYAML() (result interface{}, err error) {
	result = map[string]interface{}{
		"GCodeDesc": this.GCodeDesc,
		"XmlElement": this.XmlElement,
	}
	return
}

func LineToAbsolute(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()
	for ;len(parameters) >= 2; {
		x := parameters[0]
		y := parameters[1]
		writer.LineAbsolute(x,y)
		parameters=parameters[2:]
	}
}

func MoveAbsolute(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()
	if len(parameters) < 2 {
		return
	}
	x := parameters[0]
	y := parameters[1]
	writer.MoveAbsolute(x, y)
	LineToAbsolute(parameters[2:], writer)
}

func LineToRelative(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()
	for ;len(parameters) >= 2; {
		x := parameters[0]
		y := parameters[1]
		writer.LineRelative(x,y)
		parameters = parameters[2:]
	}
}

func MoveRelative(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()
	if len(parameters) < 2 {
		return
	}
	x := parameters[0]
	y := parameters[1]
	writer.MoveRelative(x, y)
	LineToRelative(parameters[2:], writer)
}

func DegreesToRadians(input float64) float64 {
	return input / 180 * math.Pi;
}

func AngleRadians(uX,uY,vX,vY float64) float64 {
	// Eq 5.4 https://www.w3.org/TR/SVG/implnote.html#ArcImplementationNotes
	dotProduct := uX * vX + uY * vY
	magU := math.Sqrt(uX*uX + uY*uY)
	magV := math.Sqrt(uX*uX + uY*uY)

	sign := uX * vY - uY * vX
	if sign < 0 {
		sign = -1
	} else {
		sign = 1
	}

	return sign * math.Acos(dotProduct/(magU*magV))
}

func EllipticArcAbsolute(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()

	for ;len(parameters) >= 7; {
		x1 := writer.X
		y1 := writer.Y
		rx := parameters[0]
		ry := parameters[1]
		xAxisRotation := parameters[2] // degrees (aka "phi")
		largeArcFlag := parameters[3] // 1 or 0
		sweepFlag := parameters[4] // 1 or 0
		x2 := parameters[5]
		y2 := parameters[6]

		phi := DegreesToRadians(xAxisRotation)
		fA := largeArcFlag
		fS := sweepFlag

		// Eq 5.1 https://www.w3.org/TR/SVG/implnote.html#ArcImplementationNotes
		// x1'  = | cos(phi) -sin(phi) |  * | (x1-x2)/2 |
		// y1'    | sin(phi) cos(phi)  |    | (y1-y2)/2 |
		u := (x1-x2)/2
		v := (y1-y2)/2
		x1Prime := math.Cos(phi) * u - math.Sin(phi) * v
		y1Prime := math.Sin(phi) * u + math.Cos(phi) * v

		// Eq 5.2
		// sign = fA != fS ? +1 : -1
		// cx' = sign*Sqrt( (rx^2*ry^2 - rx^2*y1Prime^2 - ry^2*x1Prime^2) / (rx^2*y1Prime^2 + ry^2*x1Prime^2) ) * |  rx*y1Prime/ry |
		// cy'                                                                                                    | -ry*x1Prime/rx |
		coeff := math.Sqrt((rx*rx*ry*ry - rx*rx*y1Prime*y1Prime - ry*ry*x1Prime*x1Prime) / (rx*rx*y1Prime*y1Prime + ry*ry*x1Prime*x1Prime))
		if fA == fS {
			coeff = -coeff
		}
		cxPrime := coeff * (rx * y1Prime / ry)
		cyPrime := coeff * (-ry * x1Prime / rx)

		// Eq 5.3
		// | cx | = | cos(phi) -sin(phi) | * | cxPrime | + | (x1+x2)/2 |
		// | cy |   | sin(phi)  cos(phi) |   | cyPrime |   | (y1+y2)/2 |
		u = (x1 + x2)/2
		v = (y1 + y2)/2
		cx := math.Cos(phi) * cxPrime - math.Sin(phi) * cyPrime + (x1+x2)/2
		cy := math.Sin(phi) * cxPrime + math.Cos(phi) * cyPrime + (y1+y2)/2


		// Eq 5.5
		v1 := (x1Prime - cxPrime)/rx
		v2 := (y1Prime - cyPrime)/ry
		theta1 := AngleRadians(1,0, v1 , v2)
		// Eq 5.6
		deltaTheta := AngleRadians(v1,v2, (-x1Prime -cxPrime)/rx, (-y1Prime - cyPrime)/ry)
		for ; deltaTheta > 2.0 * math.Pi ; {
			deltaTheta = deltaTheta - 2.0 * math.Pi
		}
		for ; deltaTheta < -(2.0 * math.Pi) ; {
			deltaTheta = deltaTheta + 2.0 * math.Pi
		}
		if sweepFlag == 0 && deltaTheta > 0 {
			deltaTheta = deltaTheta - 2.0 * math.Pi
		}
		if sweepFlag == 1 && deltaTheta < 0 {
			deltaTheta = deltaTheta + 2.0 * math.Pi
		}

		// TODO: Eq 6.1
		// TODO: Eq 6.2
		// TODO: Eq 6.3

		theta2 := theta1 + deltaTheta
		if theta1 < theta2 {
			for theta := theta1; theta < theta2; theta = theta + DegreesToRadians(10.0) {
				// Eq 3.1
				// | x | = | cos(phi) -sin(phi) | * | rx * cos(theta) | + | cx |
				// | y |   | sin(phi)  cos(phi) |   | ry * sin(theta) | + | cy |
				u = rx * math.Cos(theta)
				v = ry * math.Sin(theta)
				x := math.Cos(phi)*u - math.Sin(phi) * v + cx
				y := math.Sin(phi)*u + math.Cos(phi) * v + cy
				writer.LineAbsolute(x,y)
			}
		} else {
			for theta := theta1; theta > theta2; theta = theta - DegreesToRadians(10.0) {
				// Eq 3.1
				// | x | = | cos(phi) -sin(phi) | * | rx * cos(theta) | + | cx |
				// | y |   | sin(phi)  cos(phi) |   | ry * sin(theta) | + | cy |
				u = rx * math.Cos(theta)
				v = ry * math.Sin(theta)
				x := math.Cos(phi)*u - math.Sin(phi) * v + cx
				y := math.Sin(phi)*u + math.Cos(phi) * v + cy
				writer.LineAbsolute(x,y)
			}
		}

		writer.LineAbsolute(x2,y2)
		parameters = parameters[7:]
	}
}


func EllipticArcRelative(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()

	for ;len(parameters) >= 7; {
		rx := parameters[0]
		ry := parameters[1]
		xAxisRotation := parameters[2] // degrees
		largeArcFlag := parameters[3] // 1 or 0
		sweepFlag := parameters[4] // 1 or 0
		x := writer.X + parameters[5]
		y := writer.Y + parameters[6]

		EllipticArcAbsolute([]float64{rx,ry,xAxisRotation,largeArcFlag,sweepFlag,x,y},writer)

		parameters = parameters[7:]
	}
}

func Close(writer *GCodeWriter) {
	LineToAbsolute([]float64{writer.StartX, writer.StartY}, writer)
}

func Interpolate(start, end, t float64) float64{
	return start + t * (end - start)
}

func CubicBezierCurveAbsolute(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()

	for ;len(parameters) >= 6; {
		x0 := writer.X; _ = x0
		y0 := writer.Y; _ = y0
		x1 := parameters[0]
		y1 := parameters[1]
		x2 := parameters[2]
		y2 := parameters[3]
		x3 := parameters[4]
		y3 := parameters[5]

		// TODO: do the parameterized bezier lerp-ception thing.
		for t := 0.0; t < 1.0; t = t + 0.1 {
			xA := Interpolate(x0, x1, t)
			yA := Interpolate(y0, y1, t)
			xB := Interpolate(x1, x2, t)
			yB := Interpolate(y1, y2, t)
			xC := Interpolate(x2, x3, t)
			yC := Interpolate(y2, y3, t)

			xAB := Interpolate(xA, xB, t)
			yAB := Interpolate(yA, yB, t)
			xBC := Interpolate(xB, xC, t)
			yBC := Interpolate(yB, yC, t)

			x := Interpolate(xAB, xBC, t)
			y := Interpolate(yAB, yBC, t)

			LineToAbsolute([]float64{x,y},writer)
		}
		LineToAbsolute([]float64{x3,y3},writer)

		parameters = parameters[6:]
	}
}

func CubicBezierCurveRelative(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()

	for ;len(parameters) >= 6; {
		x0 := writer.X; _ = x0
		y0 := writer.Y; _ = y0
		x1 := writer.X + parameters[0]
		y1 := writer.Y + parameters[1]
		x2 := writer.X + parameters[2]
		y2 := writer.Y + parameters[3]
		x3 := writer.X + parameters[4]
		y3 := writer.Y + parameters[5]

		// TODO: do the parameterized bezier lerp-ception thing.
		for t := 0.0; t < 1.0; t = t + 0.1 {
			xA := Interpolate(x0, x1, t)
			yA := Interpolate(y0, y1, t)
			xB := Interpolate(x1, x2, t)
			yB := Interpolate(y1, y2, t)
			xC := Interpolate(x2, x3, t)
			yC := Interpolate(y2, y3, t)

			xAB := Interpolate(xA, xB, t)
			yAB := Interpolate(yA, yB, t)
			xBC := Interpolate(xB, xC, t)
			yBC := Interpolate(yB, yC, t)

			x := Interpolate(xAB, xBC, t)
			y := Interpolate(yAB, yBC, t)

			LineToAbsolute([]float64{x,y},writer)
		}
		LineToAbsolute([]float64{x3,y3},writer)

		parameters = parameters[6:]
	}
}


func CubicBezierSmoothCurveAbsolute(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()

	for ;len(parameters) >= 4; {
		x1 := parameters[0]
		y1 := parameters[1]
		x := parameters[2]
		y := parameters[3]

		// TODO: do the parameterized bezier lerp-ception thing.
		LineToAbsolute([]float64{x1,y1},writer)
		LineToAbsolute([]float64{x,y},writer)

		parameters = parameters[4:]
	}
}

func CubicBezierSmoothCurveRelative(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()

	for ;len(parameters) >= 4; {
		dx1 := writer.X + parameters[0];
		dy1 := writer.Y + parameters[1];
		dx := writer.X + parameters[2];
		dy := writer.Y + parameters[3];

		// TODO: do the parameterized bezier lerp-ception thing.
		LineToAbsolute([]float64{dx1,dy1},writer)
		LineToAbsolute([]float64{dx,dy},writer)

		parameters = parameters[4:]
	}
}

func HorizontalAbsolute(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()

	for ;len(parameters) >= 1; {
		x := parameters[0]
		y := writer.Y

		LineToAbsolute([]float64{x,y},writer)

		parameters = parameters[1:]
	}
}

func HorizontalRelative(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()

	for ;len(parameters) >= 1; {
		x := writer.X + parameters[0]
		y := writer.Y

		LineToAbsolute([]float64{x,y},writer)
		parameters = parameters[1:]
	}
}

func QuadraticBezierCurveAbsolute(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()

	for ;len(parameters) >= 4; {
		x0 := writer.X; _ = x0
		y0 := writer.Y; _ = y0
		x1 := parameters[0]
		y1 := parameters[1]
		x2 := parameters[2]
		y2 := parameters[3]

		// TODO: do the parameterized bezier lerp-ception thing.
		for t := 0.0; t < 1.0; t = t + 0.1 {
			xA := Interpolate(x0, x1, t)
			yA := Interpolate(y0, y1, t)
			xB := Interpolate(x1, x2, t)
			yB := Interpolate(y1, y2, t)

			x := Interpolate(xA, xB, t)
			y := Interpolate(yA, yB, t)

			LineToAbsolute([]float64{x,y}, writer)
		}
		LineToAbsolute([]float64{x2,y2},writer)

		parameters = parameters[4:]
	}
}

func QuadraticBezierCurveRelative(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()

	for ;len(parameters) >= 4; {
		x0 := writer.X
		y0 := writer.Y
		x1 := writer.X + parameters[0];
		y1 := writer.Y + parameters[1];
		x2 := writer.X + parameters[2];
		y2 := writer.Y + parameters[3];

		for t := 0.0; t < 1.0; t = t + 0.1 {
			xA := Interpolate(x0, x1, t)
			yA := Interpolate(y0, y1, t)
			xB := Interpolate(x1, x2, t)
			yB := Interpolate(y1, y2, t)

			x := Interpolate(xA, xB, t)
			y := Interpolate(yA, yB, t)

			LineToAbsolute([]float64{x,y}, writer)
		}
		LineToAbsolute([]float64{x2,y2},writer)

		parameters = parameters[4:]
	}
}


func QuadraticBezierSmoothCurveAbsolute(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()

	for ;len(parameters) >= 2; {
		x1 := parameters[0]
		y1 := parameters[1]

		// TODO: do the parameterized bezier lerp-ception thing.
		LineToAbsolute([]float64{x1,y1},writer)

		parameters = parameters[2:]
	}
}

func QuadraticBezierSmoothCurveRelative(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()

	for ;len(parameters) >= 2; {
		dx1 := writer.X + parameters[0];
		dy1 := writer.Y + parameters[1];

		// TODO: do the parameterized bezier lerp-ception thing.
		LineToAbsolute([]float64{dx1,dy1},writer)

		parameters = parameters[2:]
	}
}


func VerticalAbsolute(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()

	for ;len(parameters) >= 1; {
		x := writer.X
		y := parameters[0]

		LineToAbsolute([]float64{x,y},writer)
		parameters = parameters[1:]
	}
}

func VerticalRelative(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()

	for ;len(parameters) >= 1; {
		x := writer.X
		y := writer.Y + parameters[0]

		LineToAbsolute([]float64{x,y},writer)
		parameters = parameters[1:]
	}
}

func carveSvgCircle(input *svg.XmlElement, writer *GCodeWriter, transforms []svg.Transform) {
	defer logx.Indent(2)()

	cxStr := input.AttributeDefault("cx","0")
	cyStr := input.AttributeDefault("cy","0")
	rxStr := input.AttributeDefault("rx","auto")
	ryStr := input.AttributeDefault("ry","auto")

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
		cx+rx,cy,
		cx,cy+ry,
		cx-rx,cy,
		cx,cy-ry,
		cx+rx,cy)
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
	writer.X = 0
	writer.Y = 0
	writer.StartX = 0
	writer.StartY = 0
	writer.CommentCurrentXY("  Carving path data 0,0 is")

	pathCommands := svg.ParseSvgPathData(pathData)
	for _, command := range(pathCommands) {
		writer.Comment(fmt.Sprintf("[DEBUG] %#v\n", command))
		switch(command.Command) {
		case "A": EllipticArcAbsolute(command.Parameters, writer)
		case "a": EllipticArcRelative(command.Parameters, writer)
		case "C": CubicBezierCurveAbsolute(command.Parameters, writer)
		case "c": CubicBezierCurveRelative(command.Parameters, writer)
		case "H": HorizontalAbsolute(command.Parameters, writer)
		case "h": HorizontalRelative(command.Parameters, writer)
		case "L": LineToAbsolute(command.Parameters, writer)
		case "l": LineToRelative(command.Parameters, writer)
		case "M": MoveAbsolute(command.Parameters, writer)
		case "m": MoveRelative(command.Parameters, writer)
		case "Q": QuadraticBezierCurveAbsolute(command.Parameters, writer)
		case "q": QuadraticBezierCurveRelative(command.Parameters, writer)
		case "S": CubicBezierSmoothCurveAbsolute(command.Parameters, writer)
		case "s": CubicBezierSmoothCurveRelative(command.Parameters, writer)
		case "T": QuadraticBezierSmoothCurveAbsolute(command.Parameters, writer)
		case "t": QuadraticBezierSmoothCurveRelative(command.Parameters, writer)
		case "V": VerticalAbsolute(command.Parameters, writer)
		case "v": VerticalRelative(command.Parameters, writer)
		case "Z": Close(writer)
		case "z": Close(writer)
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
	case "": return
	case "http://www.w3.org/2000/svg:desc": return
	default:
		writer.Comment(fmt.Sprintf("this.XmlElement.id: %s this.XmlElement.Name: %#v transform: %#v\n",
			this.XmlElement.Attribute("id"),
			this.XmlElement.Name,
			transforms))
	}

	var gcodeDesc *GCodeDesc
	for element := this; gcodeDesc == nil && element != nil; element = element.Parent {
		gcodeDesc = element.GCodeDesc
	}

	if gcodeDesc != nil && gcodeDesc.CarveDepth != "" {
		writer.GCodeDesc = gcodeDesc
		writer.LiftToSafeHeight()
		writer.CurrentDepth = 0
		carveDepth := svg.MustParseNumber(gcodeDesc.CarveDepth)
		for ;writer.CurrentDepth < carveDepth; {
			writer.CurrentDepth = writer.CurrentDepth + 1 // TODO use carve increment instead.
			if writer.CurrentDepth > carveDepth {
				writer.CurrentDepth = carveDepth
			}
			writer.Comment(fmt.Sprintf("-- CurrentDepth: %f CarveDepth: %f\n", writer.CurrentDepth, carveDepth))
			switch this.XmlElement.Name {
			case "http://www.w3.org/2000/svg:path": carveSvgPath(this.XmlElement, writer, transforms)
			case "http://www.w3.org/2000/svg:circle": carveSvgCircle(this.XmlElement, writer, transforms)
			}
		}
	}

	defer logx.Indent(2)()
	for _, child := range this.Children {
		child.Carve(writer, transforms)
	}
}
