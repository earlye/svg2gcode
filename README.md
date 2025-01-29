# svg2gcode

## Overview

svg2gcode is a tool for converting svg files to gcode suitable for the
X-Carve CNC router. It uses yaml embedded in `<desc>` as its mechanism
for encoding CNC-specific data, as the intended "CAD" program is
Inkscape, and custom CSS attributes are not displayed in Inkscape in
any sensible location. The sorts of information that will fit in the
`<desc>` fields are things like "how deep should this path cut?"


## TODO:

* DONE CLI framework for reading several svg files and writing to a single output

* DONE parse svg xml, tracking full element/attribute names

* DONE traverse svg element

* DONE convert svg path data to gcode

* DONE track transforms so gcode carves in correct place

* TODO support <circle>


* TODO support <clipPath>

* TODO support <defs> and <use>

* TODO support <ellipse>

* TODO support <line>

* TODO support <pattern>

* TODO support <polygon>

* TODO support <polyline>

* TODO support <rect>

* TODO support <symbol>

* TODO support <text>

* TODO support <textPath>

* TODO support <tspan>

* TODO support <unknown> (maybe just log and fail?)

* TODO support <view>

* TODO support tabs

* TODO support ramping carve

* TODO support pocket carves in "hatch" mode

* TODO support pocket carves with arbitrary-angle hatch

* TODO generate output commands

## Similar Projects:

* https://github.com/srwiley/oksvg/tree/master

* https://github.com/srwiley/rasterx
