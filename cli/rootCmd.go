// -*- tab-width: 2 -*-
/// coverage-ignore
package cli

import (
	"encoding/xml"
	"fmt"
	"github.com/hashicorp/logutils"
	"github.com/spf13/cobra"
	"svg2gcode/svg"
	"svg2gcode/util"
	"io"
	"log"
	"os"
)

func convertSvgPath(depth string, input svg.XmlElement, output io.Writer) {
	pathData := input.Attribute(xml.Name{Local:"d"})
	log.Printf("[DEBUG] %sPath Data: %s\n", depth, pathData)

	pathCommands := svg.ParseSvgPathData(pathData)
	_ = pathCommands
}

func convertSvgElement(depth string, input svg.XmlElement, output io.Writer) {
	log.Printf("[DEBUG] %s<%s:%s>\n", depth, input.Name.Space, input.Name.Local)

	if input.Name.Space == "http://www.w3.org/2000/svg" {
		switch input.Name.Local {
		case "path": convertSvgPath(depth + "- ", input, output)
		}
	}
	
	for _, child := range input.Children {
		convertSvgElement(depth + "  ", *child, output)
	}
	log.Printf("[DEBUG] %s</%s:%s>\n", depth, input.Name.Space, input.Name.Local)
}

func convertSvg(input io.Reader, output io.Writer) (err error) {
	log.Printf("[DEBUG] input: %s output: %s\n", util.Name(input, "stdin"), util.Name(output, "stdout"))
	doc, err := svg.ParseSvgDocument(input)
	if err != nil {
		return
	}
	log.Printf("[INFO] frontmatter...")
	for _, el := range doc.FrontMatter {
		log.Printf("[INFO]   %#v\n", el)
	}
	log.Printf("[INFO] ...frontmatter done")

	log.Printf("[INFO] document: width: %s height: %s viewBox: %s\n", doc.Width(), doc.Height(), doc.ViewBox())

	convertSvgElement("", doc.XmlElement, output)
	return
}


func rootPersistentPreRunE(cmd *cobra.Command, args []string) (err error) {
	var LogLevel = cmd.Flag("verbosity").Value.String()
	filter := &logutils.LevelFilter{
		Levels: []logutils.LogLevel{"SILLY", "TRACE", "DEBUG", "WARN", "INFO", "ERROR"},
		MinLevel: logutils.LogLevel(LogLevel),
		Writer: os.Stderr,
	}
	log.SetOutput(filter)

	return nil
}

func rootRunE(cmd *cobra.Command, args []string) (err error) {
	log.Printf("[INFO] args: %#v", args)
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
		err = convertSvg(file, output)
		if err != nil {
			return
		}
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
