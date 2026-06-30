package svgx

import "svg2gcode/svg"

type GCodeDesc struct {
	OriginMarker bool   `yaml:"origin-marker,omitempty"`
	CarveDepth   string `yaml:"carve-depth,omitempty"`
	SafeHeight   string `yaml:"safe-height,omitempty"`
}

func (this *GCodeDesc) GetSafeHeight(defaultResult float64) float64 {
	mm, err := svg.ParseLengthMm(this.SafeHeight)
	if err != nil {
		return defaultResult
	}
	return mm
}

func (this *GCodeDesc) GetCarveDepth(defaultResult float64) float64 {
	mm, err := svg.ParseLengthMm(this.CarveDepth)
	if err != nil {
		return defaultResult
	}
	return mm
}
