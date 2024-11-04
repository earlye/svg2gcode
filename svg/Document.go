package svg

import (
	"encoding/xml"
)

type Document struct {
	XmlElement
	FrontMatter []xml.Token
	BackMatter []xml.Token
}

func (this *Document) Height() (string) {
	return this.Attribute(xml.Name{Local: "width"})
}

func (this *Document) Width() (string) {
	return this.Attribute(xml.Name{Local: "height"})
}

func (this *Document) ViewBox() (string) {
	return this.Attribute(xml.Name{Local: "viewBox"})
}
