package svg

import (
	"encoding/xml"
	"fmt"
)

type Matter struct {
	Tokens []xml.Token
}

func (this *Matter) Format(f fmt.State, verb rune) {
	switch(verb) {
	case 's':
		for _, el := range this.Tokens {
			fmt.Fprintf(f, "%T %s\n", el, el)
		}
	case 'v':
		fmt.Fprintf(f, "%v", this.Tokens)
	case 'q':
		fmt.Fprintf(f, "%q", this.Tokens)
	}
}

func (this *Matter) MarshalYAML() (result interface{}, err error) {
	raw := []interface{}{}

	for _, token := range(this.Tokens) {
		switch typedToken := token.(type) {
		case xml.CharData:
			raw = append(raw, string(typedToken))
		default:
			raw = append(raw, fmt.Sprintf("%T %s", token, token))
		}
	}

	result = raw
	return
}


type Document struct {
	Root XmlElement
	FrontMatter Matter
	BackMatter Matter
}

func (this *Document) Height() (string) {
	return this.Root.Attribute("width")
}

func (this *Document) Width() (string) {
	return this.Root.Attribute("height")
}

func (this *Document) ViewBox() (string) {
	return this.Root.Attribute("viewBox")
}
