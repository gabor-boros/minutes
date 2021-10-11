package utils

import (
	"bufio"
	"fmt"
	"os"
	"strings"
	"time"

	"github.com/spf13/cobra"
)

// Truncate chops the text at length and replaces the remaining with "...".
func Truncate(text string, length int) string {
	if length >= len(text) || length <= 0 {
		return text
	}

	truncated := ""
	maxLength := length - 3

	for i, char := range text {
		if i >= maxLength {
			break
		}

		truncated += string(char)
	}

	return truncated + "..."
}

// IsSliceContains checks if a string slice contains the given element or not.
func IsSliceContains(entry string, slice []string) bool {
	for _, s := range slice {
		if s == entry {
			return true
		}
	}

	return false
}

// Prompt shows the user a message and asks for input, then returns that.
func Prompt(message string) string {
	fmt.Print(message)

	reader := bufio.NewReader(os.Stdin)
	input, err := reader.ReadString('\n')
	cobra.CheckErr(err)

	return strings.TrimSpace(input)
}

// GetTime parses a string based on the given format and returns the time.
// FIXME: End and Start dates have the same value
// FIXME: Returns the UTC time, not local time
func GetTime(rawDate string, dateFormat string) (time.Time, error) {
	var date time.Time
	var err error

	if rawDate == "" {
		year, month, day := time.Now().Date()
		date = time.Date(year, month, day, 0, 0, 0, 0, time.Local)
	} else {
		return time.Parse(dateFormat, rawDate)
	}

	return date, err
}
