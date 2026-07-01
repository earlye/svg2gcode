package svgx

import "svg2gcode/svg"

type PathCursor struct {
	X, Y           float64
	StartX, StartY float64
}

type CarveCtx struct {
	PathCursor
	Z             float64
	Depth         float64
	SafeHeight    float64
	MmPerUnit     float64
	Transforms    []svg.Transform
	UsingAbsolute bool
}

type PathHandler func([]float64, CarveCtx) ([]string, CarveCtx)
