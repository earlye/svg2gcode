// -*- tab-width: 2 -*-
/// coverage-ignore
package cli

import (
	"encoding/xml"
	"fmt"
	"github.com/hashicorp/logutils"
	"github.com/spf13/cobra"
	"gopkg.in/yaml.v3"
	"svg2gcode/logx"
	"svg2gcode/svg"
	"svg2gcode/svgx"
	"svg2gcode/util"
	"io"
	"log"
	"os"
	_ "slices"
	_ "strings"
)

/*  */

func attachSvgDesc(input *svg.XmlElement, SvgxElement *svgx.SvgxElement) {
	defer logx.Indent(2)()
	// log.Printf("[SILLY] [parent: %v]\n", input.Parent)
	for _, child := range input.Children {
		switch typedToken := child.Token.(type) {
		case xml.CharData:
			log.Printf("[SILLY] CharData:\n%s\n", child.Token)
			err := yaml.Unmarshal(typedToken, &SvgxElement.GCodeDesc)
			if err != nil {
				log.Printf("[WARN] Could not read YAML from <desc> tag\n")
				log.Printf("[WARN] error: %e\n", err)
				continue
			}
			log.Printf("[SILLY] %#v\n", SvgxElement.GCodeDesc)
		}
	}
	if SvgxElement.GCodeDesc.OriginMarker {
		SvgxElement.Document.OriginMarker = input.Parent
	}
}

func svgToSvgxElement(input *svg.XmlElement, output *svgx.SvgxElement) {
	defer logx.Indent(2)()
	log.Printf("[SILLY] <%s> tokenType: %s\n",
		input.Name,
		input.TokenType,
	)

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
		svgxChild := &svgx.SvgxElement{
			Document: output.Document,
			Parent: output,
			XmlElement: child,
		}
		svgToSvgxElement(child, svgxChild)
		output.Children = append(output.Children, svgxChild)
	}

	log.Printf("[SILLY] </%s>\n", input.Name)
}

func loadSvgxDocument(input io.Reader) (result *svgx.SvgxDocument, err error) {
	defer logx.Indent(2)()
	// Load the XML, then do some post processing to handle "svgx," the CNC extensions data.
	log.Printf("[DEBUG] input: %s\n", util.FileName(input, "stdin"))
	svgDoc, err := svg.ParseSvgDocument(input)
	if err != nil {
		return
	}

	log.Printf("[SILLY] frontmatter:\n%s", &svgDoc.FrontMatter)
	log.Printf("[SILLY] document:\n%s", &svgDoc.Root)
	log.Printf("[SILLY] backmatter:\n%s", &svgDoc.BackMatter)

	log.Printf("[SILLY]\n")

	svgxRoot := svgx.SvgxElement{
		XmlElement: &svgDoc.Root,
		Parent: nil,
	}
	result = &svgx.SvgxDocument{
		Filename: util.FileName(input, "stdin"),
		SvgDocument: svgDoc,
		Root: &svgxRoot,
	}
	svgxRoot.Document = result
	svgToSvgxElement(&svgDoc.Root, &svgxRoot)

	log.Printf("[DEBUG] OriginMarker: %v\n", result.OriginMarker)
	return
}

func rootPersistentPreRunE(cmd *cobra.Command, args []string) (err error) {
	var LogLevel = cmd.Flag("verbosity").Value.String()
	filter := &logutils.LevelFilter{
		Levels: []logutils.LogLevel{"SILLY", "TRACE", "DEBUG", "WARN", "INFO", "ERROR"},
		MinLevel: logutils.LogLevel(LogLevel),
		Writer: logx.Default(),
	}
	log.SetOutput(filter)

	return nil
}

func rootRunE(cmd *cobra.Command, args []string) (err error) {
	log.Printf("[DEBUG] args: %#v", args)
	readers := []io.Reader{}
	files := []*os.File{}
	defer func() {
		for _, file := range files {
			log.Printf("[DEBUG] closing file %s\n", file.Name())
			file.Close()
		}
	} ()

	for _, filename := range args {
		log.Printf("[DEBUG] opening file %s\n", filename)
		var file *os.File
		file, err = os.Open(filename)
		if err != nil {
			return
		}
		defer file.Close()
		files = append(files, file)
		readers = append(readers, file)
	}
	if (len(readers) == 0) {
		readers = append(readers, os.Stdin)
	}

	log.Printf("[DEBUG] %d files open", len(readers))

	output := os.Stdout
	outputFilename := cmd.Flag("output").Value.String()
	if len(outputFilename) != 0 {
		output, err = os.OpenFile(outputFilename, os.O_RDWR|os.O_CREATE, 0644)
		if err != nil {
			return
		}
		files = append(files, output)
	}

	for _, file := range files {
		var svgxDoc *svgx.SvgxDocument = nil
		svgxDoc, err = loadSvgxDocument(file)
		if err != nil {
			return
		}
		log.Printf("[DEBUG] svgxDoc: %#v\n", svgxDoc)
		tmp, _ := yaml.Marshal(svgxDoc)
		log.Printf("[DEBUG] svgxDocBody: %s\n", string(tmp))

		svgxDoc.Carve(output)
	}

	return nil
}

var rootCmd = &cobra.Command{
	Use:   fmt.Sprintf("%s [flags] [...inputfile]", util.DetectName()),
	Short: fmt.Sprintf("%s helps generate gcode from svg files.", util.DetectName()),
	Long: fmt.Sprintf("%s helps generate gcode from svg [inputfile]. If no inputfile is specified, stdin is used.", util.DetectName()),
	PersistentPreRunE: rootPersistentPreRunE,
	RunE: rootRunE,
	SilenceUsage: true,
}

func init() {
	rootCmd.PersistentFlags().StringP("verbosity", "v", "WARN",
		"Display log messages with selected or greater level.\n"+
			"Valid Choices: TRACE, DEBUG, WARN, ERROR.\n"+
			"If specified without a value, uses DEBUG\n"+
			"Examples: svg2gcode -v=ERROR // 'only show errors'\n"+
			"          svg2gcode -v       // 'show debug logs, warnings, and errors'\n"+
			"          svg2gcode -v=TRACE // 'show all logs'\n" )

	rootCmd.PersistentFlags().Lookup("verbosity").NoOptDefVal = "DEBUG"

	rootCmd.Flags().StringP("output", "o", "",
		"Output filename. If not provided, output goes to stdout")
}
