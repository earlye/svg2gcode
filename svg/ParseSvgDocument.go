package svg

import (
	"encoding/xml"
	"fmt"
	_ "github.com/go-errors/errors"
	"golang.org/x/net/html/charset"
	"io"
	"log"
	"strings"
)

func (this* XmlElement) Decode(startElement xml.StartElement, decoder *xml.Decoder) (err error) {
	log.Printf("[TRACE] XmlElement::Decode: element: %#v", startElement.Name)
	// copy startElement into this
	this.Name = startElement.Name
	this.Attributes = map[xml.Name]string{}
	for _, attr := range startElement.Attr {
		this.Attributes[attr.Name] = attr.Value
	}

	log.Printf("[TRACE] XmlElement::Decode: Decoding children of: %#v", startElement.Name)
	// recursive. You know, for the children. Get it? For the _children_!
	for {
		var token xml.Token
		token, err = decoder.Token()
		log.Printf("[TRACE] XmlElement::Decode: token: %#v err: %#v", token, err)
		if err != nil {
			return
		}
		
		switch typedToken := token.(type) {
		case xml.StartElement:
			log.Printf("[TRACE] XmlElement::Decode: xml.StartElement: %s Parent: %s", typedToken.Name, this.Name)
			child := &XmlElement{}
			child.Parent = this
			child.Token = xml.CopyToken(typedToken)
			child.Decode(typedToken, decoder)
			this.Children = append(this.Children, child)
		case xml.EndElement:
			log.Printf("[TRACE] XmlElement::Decode: xml.EndElement: %#v", typedToken.Name)
			// It is not necessary to check the validity, as xml.Decoder does this for us.
			return
		default:
			child := &XmlElement{}
			child.Parent = this
			child.Token = xml.CopyToken(child)
			this.Children = append(this.Children, child)
		}
	}
}

type DecoderTokenFunction = func(decoder *xml.Decoder) (xml.Token, error);
func xmlDecoderToken(decoder *xml.Decoder) (xml.Token, error) {
	log.Printf("[TRACE] xmlDecoderToken(decoder: %p)\n", decoder)
	return decoder.Token()
}

func DecodeSvgDocument(decoder *xml.Decoder, decoderToken DecoderTokenFunction) (result Document, err error) {
	for {
		var token xml.Token
		token, err = decoderToken(decoder)
		if err == io.EOF {
			err = fmt.Errorf("input was empty");
			return
		}
		if err != nil {
			return
		}
		switch typedToken := token.(type) {
		case xml.StartElement:
			log.Printf("[TRACE] DecodeSvgDocument: xml.StartElement: %s", typedToken.Name)
			if typedToken.Name.Local != "svg" {
				err = fmt.Errorf("Unexpected top-level token")
				return
			}
			if typedToken.Name.Space != "" && 
				typedToken.Name.Space != "http://www.w3.org/2000/svg" {
				err = fmt.Errorf("Unexpected namespace")
				return
			}
			err = result.XmlElement.Decode(typedToken, decoder)
			return // HAPPY PATH
		case xml.CharData:
			log.Printf("[TRACE] DecodeSvgDocument: xml.CharData: %s", typedToken);
			result.FrontMatter = append(result.FrontMatter, xml.CopyToken(token))
		case xml.ProcInst:
			log.Printf("[TRACE] DecodeSvgDocument: xml.ProcInst: %s", typedToken);
			result.FrontMatter = append(result.FrontMatter, xml.CopyToken(token))
		case xml.Comment:
			log.Printf("[TRACE] DecodeSvgDocument: xml.Comment: %s", typedToken);
			result.FrontMatter = append(result.FrontMatter, xml.CopyToken(token))
		default:
			log.Printf("[ERROR] DecodeSvgDocument: unexpected token type: %#v\n", token) /// cover: unreachable
			err = fmt.Errorf("unexpected token type") /// cover: unreachable
			return /// cover: unreachable
		}
	}
}

func DecodeSvgBackmatter(result *Document, decoder *xml.Decoder, decoderToken DecoderTokenFunction) (err error) {
	for {
		var token xml.Token
		token, err = decoderToken(decoder)
		if err == io.EOF {
			err = nil
			return
		}
		switch typedToken := token.(type) {
		case xml.CharData:
			log.Printf("[TRACE] DecodeSvgBackmatter: xml.CharData: %s", typedToken);
			result.BackMatter = append(result.BackMatter, xml.CopyToken(token))
		case xml.ProcInst:
			log.Printf("[TRACE] DecodeSvgBackmatter: xml.ProcInst: %s", typedToken);
			result.BackMatter = append(result.BackMatter, xml.CopyToken(token))
		case xml.Comment:
			log.Printf("[TRACE] DecodeSvgBackmatter: xml.Comment: %s", typedToken);
			result.BackMatter = append(result.BackMatter, xml.CopyToken(token))
		default:
			log.Printf("[ERROR] DecodeSvgBackmatter: unexpected token type: %#v\n", token) /// cover: unreachable
			err = fmt.Errorf("unexpected token type") /// cover: unreachable
			return /// cover: unreachable
		}
	}
	
}

func ParseSvgDocument(input io.Reader) (result Document, err error) {
	decoder := xml.NewDecoder(input)
	decoder.CharsetReader = charset.NewReaderLabel

	result, err = DecodeSvgDocument(decoder, xmlDecoderToken)
	if err != nil {
		return
	}

	err = DecodeSvgBackmatter(&result, decoder, xmlDecoderToken)
	return
}


func ParseSvgString(input string) (result Document, err error) {
	reader := strings.NewReader(input)
	return ParseSvgDocument(reader)
}
