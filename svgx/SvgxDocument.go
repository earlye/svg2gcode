package svgx

import (
	_ "encoding/xml"
	"fmt"
	"svg2gcode/svg"
	"io"
)

type GCodeWriter struct {
	Writer io.Writer
}

func (this *GCodeWriter) Write(input string) {
	this.Writer.Write([]byte(input))
}

func (this *GCodeWriter) Comment(input string) {
	this.Write("; " + input)
}

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
	writer := GCodeWriter{
		Writer: output,
	}
	writer.Comment(fmt.Sprintf("Source: %s\n", this.Filename))

}
