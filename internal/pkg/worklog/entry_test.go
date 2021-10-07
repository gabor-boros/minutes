package worklog_test

import (
	"testing"
	"time"

	"github.com/gabor-boros/minutes/internal/pkg/worklog"
	"github.com/stretchr/testify/assert"
)

func getTestEntry() worklog.Entry {
	start := time.Date(2021, 10, 2, 5, 0, 0, 0, time.Local)
	end := start.Add(time.Hour * 2)

	return worklog.Entry{
		Client: worklog.IDNameField{
			ID:   "client-id",
			Name: "My Awesome Company",
		},
		Project: worklog.IDNameField{
			ID:   "project-id",
			Name: "Internal projects",
		},
		Task: worklog.IDNameField{
			ID:   "task-id",
			Name: "TASK-0123",
		},
		Summary:            "Write worklog transfer CLI tool",
		Notes:              "It is a lot easier than expected",
		Start:              start,
		BillableDuration:   end.Sub(start),
		UnbillableDuration: 0,
	}
}

func TestIDNameFieldIsComplete(t *testing.T) {
	var field worklog.IDNameField

	assert.False(t, field.IsComplete())

	field = worklog.IDNameField{
		ID: "101",
	}
	assert.False(t, field.IsComplete())

	field = worklog.IDNameField{
		ID:   "101",
		Name: "MARVEL-101",
	}
	assert.True(t, field.IsComplete())
}

func TestEntryKey(t *testing.T) {
	entry := getTestEntry()
	assert.Equal(t, "Internal projects:TASK-0123:Write worklog transfer CLI tool:2021-10-02", entry.Key())
}

func TestEntryIsComplete(t *testing.T) {
	entry := getTestEntry()
	assert.True(t, entry.IsComplete())
}

func TestEntryIsCompleteIncomplete(t *testing.T) {
	var entry worklog.Entry

	entry = getTestEntry()
	entry.Client = worklog.IDNameField{}
	assert.False(t, entry.IsComplete())

	entry = getTestEntry()
	entry.Project = worklog.IDNameField{}
	assert.False(t, entry.IsComplete())

	entry = getTestEntry()
	entry.Task = worklog.IDNameField{}
	assert.False(t, entry.IsComplete())

	entry = getTestEntry()
	entry.Summary = ""
	assert.False(t, entry.IsComplete())

	entry = getTestEntry()
	entry.Start = time.Time{}
	assert.False(t, entry.IsComplete())

	entry = getTestEntry()
	entry.BillableDuration = 0
	entry.UnbillableDuration = 0
	assert.False(t, entry.IsComplete())
}
