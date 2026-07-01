package svgx

import (
	"encoding/xml"
	"io"
	"log"
	"svg2gcode/logx"
	"svg2gcode/svg"
	"svg2gcode/util"

	"gopkg.in/yaml.v3"
)

func attachSvgDesc(input *svg.XmlElement, element *SvgxElement) {
	defer logx.Indent(2)()
	for _, child := range input.Children {
		switch typedToken := child.Token.(type) {
		case xml.CharData:
			log.Printf("[SILLY] CharData:\n%s\n", child.Token)
			err := yaml.Unmarshal(typedToken, &element.GCodeDesc)
			if err != nil {
				log.Printf("[WARN] Could not read YAML from <desc> tag\n")
				log.Printf("[WARN] error: %e\n", err)
				continue
			}
			log.Printf("[SILLY] %#v\n", element.GCodeDesc)
		}
	}
	if element.GCodeDesc != nil && element.GCodeDesc.OriginMarker {
		element.Document.OriginMarker = input.Parent
	}
}

func svgToSvgxElement(input *svg.XmlElement, output *SvgxElement) {
	defer logx.Indent(2)()
	log.Printf("[SILLY] <%s> tokenType: %s\n", input.Name, input.TokenType)

	switch input.Name {
	case "http://www.w3.org/2000/svg:desc":
		log.Printf("[SILLY] <desc> input: %p output: %p", input, output)
		log.Printf("[SILLY] ... output.parent: %p", output.Parent)
		attachSvgDesc(input, output.Parent)
	case "http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd:namedview":
		break
	case "":
		break
	}

	for _, child := range input.Children {
		svgxChild := &SvgxElement{
			Document:   output.Document,
			Parent:     output,
			XmlElement: child,
		}
		svgToSvgxElement(child, svgxChild)
		output.Children = append(output.Children, svgxChild)
	}

	log.Printf("[SILLY] </%s>\n", input.Name)
}

// computeEffectiveDescs propagates GCodeDesc from parent to child so Carve
// can read EffectiveDesc directly instead of walking the parent chain.
func computeEffectiveDescs(element *SvgxElement, inherited *GCodeDesc) {
	effective := inherited
	if element.GCodeDesc != nil {
		effective = element.GCodeDesc
	}
	element.EffectiveDesc = effective
	for _, child := range element.Children {
		computeEffectiveDescs(child, effective)
	}
}

// computeMmPerUnit derives the physical-to-user-unit scale from the SVG root element.
// Falls back to 1.0 (1 user unit = 1 mm) when dimensions or viewBox are absent.
func computeMmPerUnit(doc *svg.Document) float64 {
	viewBox, err := svg.ParseViewBox(doc.ViewBox())
	if err != nil || viewBox.Width == 0 {
		return 1.0
	}
	widthMm, err := svg.ParseLengthMm(doc.Width())
	if err != nil || widthMm == 0 {
		return 1.0
	}
	return widthMm / viewBox.Width
}

// LoadDocument parses an SVG reader and returns a fully-annotated SvgxDocument.
func LoadDocument(input io.Reader) (result *SvgxDocument, err error) {
	defer logx.Indent(2)()
	log.Printf("[DEBUG] input: %s\n", util.FileName(input, "stdin"))
	svgDoc, err := svg.ParseSvgDocument(input)
	if err != nil {
		return
	}

	log.Printf("[SILLY] frontmatter:\n%s", &svgDoc.FrontMatter)
	log.Printf("[SILLY] document:\n%s", &svgDoc.Root)
	log.Printf("[SILLY] backmatter:\n%s", &svgDoc.BackMatter)
	log.Printf("[SILLY]\n")

	svgxRoot := SvgxElement{
		XmlElement: &svgDoc.Root,
		Parent:     nil,
	}
	result = &SvgxDocument{
		Filename:    util.FileName(input, "stdin"),
		SvgDocument: svgDoc,
		Root:        &svgxRoot,
		MmPerUnit:   computeMmPerUnit(svgDoc),
	}
	svgxRoot.Document = result
	svgToSvgxElement(&svgDoc.Root, &svgxRoot)
	computeEffectiveDescs(&svgxRoot, nil)

	log.Printf("[DEBUG] OriginMarker: %v\n", result.OriginMarker)
	return
}
