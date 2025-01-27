package svgx

import (
	"fmt"
	"svg2gcode/logx"
	"svg2gcode/svg"
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
		writer.LineAbsolute(x,y,0)
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
	writer.MoveAbsolute(x, y, 0)
	LineToAbsolute(parameters[2:], writer)
}

func LineToRelative(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()
	for ;len(parameters) >= 2; {
		x := parameters[0]
		y := parameters[1]
		writer.LineRelative(x,y,0)
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
	writer.MoveRelative(x, y, 0)
	LineToRelative(parameters[2:], writer)
}

func EllipticArcAbsolute(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()

	for ;len(parameters) >= 7; {
		rx := parameters[0]
		ry := parameters[1]
		xAxisRotation := parameters[2] // degrees
		largeArcFlag := parameters[3] // 1 or 0
		sweepFlag := parameters[4] // 1 or 0
		x := parameters[5]
		y := parameters[6]

		_ = rx+ry+xAxisRotation+largeArcFlag+sweepFlag

		writer.LineAbsolute(x,y,0)
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
		x := parameters[5]
		y := parameters[6]

		_ = rx+ry+xAxisRotation+largeArcFlag+sweepFlag

		writer.LineRelative(x,y,0)
		parameters = parameters[7:]
	}
}

func Close(writer *GCodeWriter) {
	LineToAbsolute([]float64{writer.StartX, writer.StartY}, writer)
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
		x := parameters[4]
		y := parameters[5]

		// TODO: do the parameterized bezier lerp-ception thing.
		LineToAbsolute([]float64{x1,y1},writer)
		LineToAbsolute([]float64{x2,y2},writer)
		LineToAbsolute([]float64{x,y},writer)

		parameters = parameters[6:]
	}
}

func CubicBezierCurveRelative(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()

	for ;len(parameters) >= 6; {
		dx1 := writer.X + parameters[0];
		dy1 := writer.Y + parameters[1];
		dx2 := writer.X + parameters[2];
		dy2 := writer.Y + parameters[3];
		dx := writer.X + parameters[4]
		dy := writer.Y + parameters[5]

		// TODO: do the parameterized bezier lerp-ception thing.
		LineToAbsolute([]float64{dx1,dy1},writer)
		LineToAbsolute([]float64{dx2,dy2},writer)
		LineToAbsolute([]float64{dx,dy},writer)

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
		LineToAbsolute([]float64{x1,y1},writer)
		LineToAbsolute([]float64{x2,y2},writer)

		parameters = parameters[4:]
	}
}

func QuadraticBezierCurveRelative(parameters []float64, writer *GCodeWriter) {
	defer logx.Indent(2)()

	for ;len(parameters) >= 4; {
		dx1 := writer.X + parameters[0];
		dy1 := writer.Y + parameters[1];
		dx2 := writer.X + parameters[2];
		dy2 := writer.Y + parameters[3];

		// TODO: do the parameterized bezier lerp-ception thing.
		LineToAbsolute([]float64{dx1,dy1},writer)
		LineToAbsolute([]float64{dx2,dy2},writer)

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


func carveSvgPath(input *svg.XmlElement, writer *GCodeWriter, transforms []svg.Transform)  {
	defer logx.Indent(2)()

	pathData := input.Attribute("d")
	writer.Comment(fmt.Sprintf("-- Carving path Data: %s\n", pathData))
	writer.SetTransforms(transforms)
	writer.X = 0
	writer.Y = 0
	writer.Z = 0
	writer.StartX = 0
	writer.StartY = 0
	writer.StartZ = 0

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
	_ = pathCommands
}

func (this *SvgxElement) Carve(writer *GCodeWriter, transforms []svg.Transform) {

	elementTransformStr := this.XmlElement.Attribute("transform")
	if len(elementTransformStr) != 0 {
		elementTransform, _ := svg.ParseTransformList(elementTransformStr)
		transforms = append(transforms, elementTransform...)
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

	if gcodeDesc != nil {
		if gcodeDesc.CarveDepth != "" {
			writer.Comment(fmt.Sprintf("-- Need to carve to depth: %s\n", gcodeDesc.CarveDepth))
			switch this.XmlElement.Name {
			case "http://www.w3.org/2000/svg:path": carveSvgPath(this.XmlElement, writer, transforms)
			}
		}
	}

	defer logx.Indent(2)()
	for _, child := range this.Children {
		child.Carve(writer, transforms)
	}
}
