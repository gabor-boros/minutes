package worklog

import (
	"fmt"
	"math"
	"regexp"
	"time"
)

// IDNameField stands for every field that has an ID and Name.
type IDNameField struct {
	ID   string `json:"id"`
	Name string `json:"name"`
}

// IsComplete indicates if the field has both ID and Name filled.
// In case both fields are filled, it returns true, otherwise, false.
func (f IDNameField) IsComplete() bool {
	return f.ID != "" && f.Name != ""
}

// Entry represents the worklog entry and contains all the necessary data.
type Entry struct {
	Client             IDNameField
	Project            IDNameField
	Task               IDNameField
	Summary            string
	Notes              string
	Start              time.Time
	BillableDuration   time.Duration
	UnbillableDuration time.Duration
}

// Key returns a unique, per entry key used for grouping similar entries.
func (e *Entry) Key() string {
	return fmt.Sprintf("%s:%s:%s:%s", e.Project.Name, e.Task.Name, e.Summary, e.Start.Format("2006-01-02"))
}

// IsComplete indicates if the entry has all the necessary fields filled.
// If all the necessary fields are complete it returns true, otherwise, false.
func (e *Entry) IsComplete() bool {
	hasClient := e.Client.IsComplete()
	hasProject := e.Project.IsComplete()
	hasTask := e.Task.IsComplete()

	isMetadataFilled := hasProject && hasClient && hasTask && e.Summary != ""
	isTimeFilled := !e.Start.IsZero() && (e.BillableDuration.Seconds() > 0 || e.UnbillableDuration.Seconds() > 0)

	return isMetadataFilled && isTimeFilled
}

// SplitDuration splits the billable and unbillable duration to N parts.
func (e *Entry) SplitDuration(parts int) (splitBillableDuration time.Duration, splitUnbillableDuration time.Duration) {
	splitBillableDuration = time.Duration(math.Round(float64(e.BillableDuration.Nanoseconds()) / float64(parts)))
	splitUnbillableDuration = time.Duration(math.Round(float64(e.UnbillableDuration.Nanoseconds()) / float64(parts)))
	return splitBillableDuration, splitUnbillableDuration
}

// SplitByTagsAsTasks splits the entry into pieces treating tags as tasks.
// Not matching tags won't be treated as a new entry should be created,
// therefore that tag will be skipped and the returned entries will lack that.
func (e *Entry) SplitByTagsAsTasks(summary string, regex *regexp.Regexp, tags []IDNameField) []Entry {
	var tasks []IDNameField
	for _, tag := range tags {
		if taskName := regex.FindString(tag.Name); taskName != "" {
			tasks = append(tasks, tag)
		}
	}

	var entries []Entry
	totalTasks := len(tasks)

	for _, task := range tasks {
		splitBillable, splitUnbillable := e.SplitDuration(totalTasks)

		entries = append(entries, Entry{
			Client:             e.Client,
			Project:            e.Project,
			Task:               task,
			Summary:            summary,
			Notes:              e.Notes,
			Start:              e.Start,
			BillableDuration:   splitBillable,
			UnbillableDuration: splitUnbillable,
		})
	}

	return entries
}
