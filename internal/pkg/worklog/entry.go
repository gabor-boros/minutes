package worklog

import (
	"fmt"
	"time"
)

// IDNameField stands for every field that has an ID and Name.
type IDNameField struct {
	ID   string
	Name string
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
