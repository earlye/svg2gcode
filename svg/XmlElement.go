package svg

import (
	"encoding/xml"
	"fmt"
)

type XmlElement struct {
	Name       string
	TokenType  string
	Token      xml.Token `yaml:"-"`
	// GCodeDesc  svgx.GCodeDesc
	Attributes map[string]string
	Parent     *XmlElement `yaml:"-"`
	Children   []*XmlElement
	Document   *Document `yaml:"-"`
}

func (this *XmlElement) Attribute(name string) (result string) {
	result = ""
	result, _ = this.Attributes[name]
	return
}

func (this *XmlElement) String() (result string) {
	result = fmt.Sprintf("<%s>", this.Name)
	return
}

/* func (this *XmlElement) MarshalYAML() (result interface{}, err error) {
	raw := map[string]interface{}{
		"Name": this.Name,
		"TokenType": this.TokenType,
		"Attributes": this.Attributes,
		"Children": this.Children,
	}
	if this.TokenType == "xml.CharData" {
		raw["CharData"] = string(this.Token.(xml.CharData))
	}
	result = raw
	return
        } */
