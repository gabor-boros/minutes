package utils

import (
	"time"

	"github.com/jedib0t/go-pretty/v6/progress"
)

// NewProgressWriter returns a pre-configured progress writer.
func NewProgressWriter(updateFrequency time.Duration) progress.Writer {
	writer := progress.NewWriter()

	writer.ShowTime(true)
	writer.ShowTracker(false)
	writer.ShowValue(false)

	writer.SetAutoStop(true)
	writer.SetTrackerPosition(progress.PositionRight)

	writer.SetMessageWidth(50)
	writer.SetUpdateFrequency(updateFrequency)

	writer.Style().Colors = progress.StyleColorsDefault
	writer.Style().Options.DoneString = "uploaded!"
	writer.Style().Options.ErrorString = "failed!  " // Have the same length as DoneString
	writer.Style().Options.Separator = "\t"
	writer.Style().Options.SnipIndicator = "..."

	return writer
}
