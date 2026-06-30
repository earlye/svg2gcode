package svgx

import (
	"fmt"
	"io"
	"strings"
	"svg2gcode/svg"
)

type GCodeWriter struct {
	Ctx    CarveCtx
	Writer io.Writer
}

func (this *GCodeWriter) Write(input string) {
	this.Writer.Write([]byte(input))
}

func (this *GCodeWriter) Comment(input string) {
	this.Write("; " + strings.Replace(input, "\n", "\\n", -1) + "\n")
}

func (this *GCodeWriter) SetTransforms(input []svg.Transform) {
	this.Ctx.Transforms = input
	this.Write(fmt.Sprintf("; Using transforms: %#v\n", input))
}

func (this *GCodeWriter) CommentCurrentXY(input string) {
	tx, ty := svg.ApplyTransformList(this.Ctx.X, this.Ctx.Y, this.Ctx.Transforms)
	tx *= this.Ctx.MmPerUnit
	ty *= this.Ctx.MmPerUnit
	this.Comment(fmt.Sprintf("%s %f %f\n", input, tx, ty))
}

func (this *GCodeWriter) LiftToSafeHeight() {
	lines, newCtx := liftToSafeHeightLines(this.Ctx)
	this.Ctx = newCtx
	for _, line := range lines {
		this.Write(line)
	}
}
