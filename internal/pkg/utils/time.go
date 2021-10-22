package utils

import "time"

// DateFormat is the enumeration of available date formats, used by clients.
// Although the builtin time package contains several formatting options, some
// clients are using nonsense date time formats.
type DateFormat int

const (
	// DateFormatISO8601 represents the ISO 8601 date format.
	DateFormatISO8601 DateFormat = iota
	DateFormatRFC3339
	// DateFormatRFC3339Compact is similar to RFC3339, but has no separation.
	// This is not a standard date time format, it is used by Timewarrior.
	DateFormatRFC3339Compact
	// DateFormatRFC3339Local is similar to RFC3339, but lacks timezone info.
	// This is not a standard date time format, it is used by Timewarrior.
	DateFormatRFC3339Local
)

// String returns the string representation of the format.
func (d DateFormat) String() string {
	return []string{
		"2006-01-02",          // DateFormatISO8601
		time.RFC3339,          // DateFormatRFC3339
		"20060102T150405Z",    // DateFormatRFC3339Compact
		"2006-01-02T15:04:05", // DateFormatRFC3339Local
	}[d]
}

// Format returns the formatted version of the given time.
func (d DateFormat) Format(t time.Time) string {
	return t.Format(d.String())
}
