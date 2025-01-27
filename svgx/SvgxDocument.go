package svgx

import (
	_ "encoding/xml"
	"fmt"
	"svg2gcode/svg"
	"io"
)

type SvgxDocument struct {
	Filename string
	SvgDocument *svg.Document
	Root *SvgxElement
	OriginMarker *svg.XmlElement
}

func (this *SvgxDocument) MarshalYAML() (result interface{}, err error) {
	raw := map[string]interface{}{
		"Filename": this.Filename,
		"FrontMatter" : &this.SvgDocument.FrontMatter,
		"BackMatter" : &this.SvgDocument.BackMatter,
		"Root": this.Root,
	}
	if this.OriginMarker != nil {
		raw["OriginMarkerId"] = this.OriginMarker.Attribute("id")
	}

	result = raw
	return
}

func (this *SvgxDocument) Carve(output io.Writer) {
	writer := &GCodeWriter{
		Writer: output,
	}
	writer.Comment(fmt.Sprintf("Source: %s\n", this.Filename))
	transform := []svg.Transform{svg.Transform{
		Name: "scale",
		Parameters: []float64{1,-1},
	}}

	if this.OriginMarker != nil {
		writer.Comment(fmt.Sprintf("Origin: (%v,%v)\n", this.OriginMarker.Attribute("cx"), this.OriginMarker.Attribute("cy")))
		transform = append(transform, svg.Transform{
			Name: "translate",
			Parameters: []float64{
				-svg.MustParseNumber(this.OriginMarker.Attribute("cx")),
				-svg.MustParseNumber(this.OriginMarker.Attribute("cy")),
			},
		})
	}

	this.Root.Carve(writer, transform)
}
