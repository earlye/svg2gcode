package svg

import (
	"encoding/xml"
)

type XmlElement struct {
	Name       xml.Name
	Token      xml.Token
	Attributes map[xml.Name]string
	Parent     *XmlElement
	Children   []*XmlElement
}

func (this *XmlElement) Attribute(name xml.Name) (result string) {
	result = ""
	result, _ = this.Attributes[name]
	return
}
