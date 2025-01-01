package svgx

type GCodeDesc struct {
	OriginMarker bool `yaml:"origin-marker,omitempty"`
	CarveDepth string `yaml:"carve-depth,omitempty"`
	SafeHeight string `yaml:"safe-height,omitempty"`
}
