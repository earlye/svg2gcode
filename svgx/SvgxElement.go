package svgx

import (
	"svg2gcode/svg"
)

type SvgxElement struct {
	GCodeDesc GCodeDesc
	Document *SvgxDocument `yaml:"omit"`
	Parent *SvgxElement `yaml:"omit"`
	XmlElement *svg.XmlElement `yaml:"omit"`
}

func (this *SvgxElement) MarshalYAML() (result interface{}, err error) {
	result = map[string]interface{}{
		"GCodeDesc": this.GCodeDesc,
		"XmlElement": this.XmlElement,
	}
	return
}
