package svgx

import (
	"fmt"
	"io"
	"strings"
	"svg2gcode/svg"
)

type GCodeWriter struct {
	UsingAbsolute bool
	X,Y,Z float64
	StartX, StartY, StartZ float64
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

func (this *GCodeWriter) MoveAbsolute(x,y,z float64) {
	this.UseAbsolute()
	// TODO: Raise to safe height
	this.X = x
	this.Y = y
	this.Z = z
	this.StartX = x
	this.StartY = y
	this.StartZ = z
	tx, ty := svg.ApplyTransformList(this.X, this.Y, this.Transforms)
	this.Write(fmt.Sprintf("G0 F1000 X%f Y%f Z%f; (move-absolute: %f[%f],%f[%f],%f)\n",
		tx, ty, this.Z,
		this.X, x, this.Y, y, z))
}

func (this *GCodeWriter) MoveRelative(x,y,z float64) {
	this.UseAbsolute()
	// TODO: Raise to safe height
	this.X = this.X + x
	this.Y = this.Y + y
	this.Z = this.Z + z
	this.StartX = this.X
	this.StartY = this.Y
	this.StartZ = this.Z
	tx, ty := svg.ApplyTransformList(this.X, this.Y, this.Transforms)
	this.Write(fmt.Sprintf("G0 F1000 X%f Y%f Z%f; (move-relative: %f,%f,%f)\n",
		tx, ty, this.Z,
		x, y, z))
}

func (this *GCodeWriter) LineAbsolute(x,y,z float64) {
	this.UseAbsolute()
	this.X = x
	this.Y = y
	this.Z = z
	tx, ty := svg.ApplyTransformList(this.X, this.Y, this.Transforms)
	this.Write(fmt.Sprintf("G1 F1000 X%f Y%f Z%f; (line-absolute: %f,%f,%f)\n",
		tx, ty, this.Z,
		x, y, z))
}

func (this *GCodeWriter) LineRelative(x,y,z float64) {
	this.UseAbsolute()
	this.X = this.X + x
	this.Y = this.Y + y
	this.Z = this.Z + z
	tx, ty := svg.ApplyTransformList(this.X, this.Y, this.Transforms)
	this.Write(fmt.Sprintf("G1 F1000 X%f Y%f Z%f; (line-relative: %f,%f,%f)\n",
		tx, ty, this.Z,
		x, y, z))
}
