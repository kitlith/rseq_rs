# rseq_rs

TBH I still expect some breakage from the current code, so make sure to make a
GitHub issue on crashes or things not working as expected.

# Usage

## Disassemble
`disassemble input.brseq output.txt` where input is a file in the BRSEQ format and
output is where you wish its disassembly to be output.

## Assemble
`assemble input.txt output.brseq` where input is an 'assembly' file in the format
produced by disassembly, and output is where you wish the resulting BRSEQ to
end up.

## Invert
`invert input.brseq output.brseq` where input is a BRSEQ file and output is where you
want to create a BRSEQ with 'inverted' notes.
(for every note it does `0x7F - note_value`)

This was created as a very simple test for producing songs that could be used
to replace other songs before the assembler was created/finished.

## Play
`play input.brseq output.midi`

This is essentially a reimplementation of rseq2midi using the same parsing code
as the rest of these programs. YMMV, but last I checked it handled call and jump better
than rseq2midi.


# Credits
Atlas, for the BRSEQ documentation that was immensely useful for implementing this (https://pastebin.com/xgsKecv9)
Ruben Nunez and Valley Bell, for making rseq2midi which `play` is based on and was also used as a form of documentation
