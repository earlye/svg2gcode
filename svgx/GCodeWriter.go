package svgx

import (
	"fmt"
	"io"
	"strings"
	"svg2gcode/svg"
)

type GCodeWriter struct {
	UsingAbsolute bool
	GCodeDesc *GCodeDesc
	CurrentDepth float64
	X,Y,Z float64
	StartX, StartY float64
	SafeHeight float64
	Writer io.Writer
	Transforms []svg.Transform
}

func (this *GCodeWriter) UseAbsolute() {
	if this.UsingAbsolute {
		return
	}
	this.Write("G90 ; Use absolute positioning.\n")
	this.UsingAbsolute = true
}

func (this *GCodeWriter) SetTransforms(input []svg.Transform) {
	this.Transforms = input
	this.Write(fmt.Sprintf("; Using transforms: %#v\n", input))
}

func (this *GCodeWriter) Write(input string) {
	this.Writer.Write([]byte(input))
}

func (this *GCodeWriter) Comment(input string) {
	this.Write("; " + strings.Replace(input,"\n","\\n",-1) + "\n")
}

func (this *GCodeWriter) LiftToSafeHeight() {
	this.SafeHeight = this.GCodeDesc.GetSafeHeight(10)
	if this.Z == this.SafeHeight {
		return
	}
	tx, ty := svg.ApplyTransformList(this.X, this.Y, this.Transforms)
	this.Write(fmt.Sprintf("G0 F1000 X%f Y%f Z%f; (lift tosafe-height)\n",
		tx, ty, this.SafeHeight))
	this.Z = this.SafeHeight
}

func (this *GCodeWriter) CommentCurrentXY(input string) {
	tx, ty := svg.ApplyTransformList(this.X, this.Y, this.Transforms)
	this.Comment(fmt.Sprintf("%s %f %f\n",
		input, tx, ty))
}

func (this *GCodeWriter) MoveAbsolute(x,y float64) {
	this.UseAbsolute()
	this.LiftToSafeHeight()
	this.X = x
	this.Y = y
	this.StartX = x
	this.StartY = y
	tx, ty := svg.ApplyTransformList(this.X, this.Y, this.Transforms)
	this.Write(fmt.Sprintf("G0 F1000 X%f Y%f Z%f; (move-absolute: %f[%f],%f[%f] - safe-height)\n",
		tx, ty, this.SafeHeight,
		this.X, x, this.Y, y))
	this.Write(fmt.Sprintf("G0 F1000 X%f Y%f Z%f; (move-absolute: %f[%f],%f[%f])\n",
		tx, ty, this.CurrentDepth,
		this.X, x, this.Y, y))
	this.Z = this.CurrentDepth
}

func (this *GCodeWriter) MoveRelative(x,y float64) {
	this.UseAbsolute()
	this.LiftToSafeHeight()
	this.X = this.X + x
	this.Y = this.Y + y
	this.StartX = this.X
	this.StartY = this.Y
	tx, ty := svg.ApplyTransformList(this.X, this.Y, this.Transforms)
	this.Write(fmt.Sprintf("G0 F1000 X%f Y%f Z%f; (move-relative: %f,%f - safe-height)\n",
		tx, ty, this.SafeHeight, x, y))
	this.Write(fmt.Sprintf("G0 F1000 X%f Y%f Z%f; (move-relative: %f,%f)\n",
		tx, ty, this.CurrentDepth,
		x, y))
	this.Z = this.CurrentDepth
}

func (this *GCodeWriter) LineAbsolute(x,y float64) {
	this.UseAbsolute()
	this.X = x
	this.Y = y
	tx, ty := svg.ApplyTransformList(this.X, this.Y, this.Transforms)
	this.Write(fmt.Sprintf("G1 F1000 X%f Y%f Z%f; (line-absolute: %f,%f)\n",
		tx, ty, this.CurrentDepth,
		x, y))
	this.Z = this.CurrentDepth
}

func (this *GCodeWriter) LineRelative(x,y float64) {
	this.UseAbsolute()
	this.X = this.X + x
	this.Y = this.Y + y
	tx, ty := svg.ApplyTransformList(this.X, this.Y, this.Transforms)
	this.Write(fmt.Sprintf("G1 F1000 X%f Y%f Z%f; (line-relative: %f,%f)\n",
		tx, ty, this.CurrentDepth,
		x, y))
	this.Z = this.CurrentDepth
}
