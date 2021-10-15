package worklog_test

import (
	"regexp"
	"testing"
	"time"

	"github.com/stretchr/testify/require"

	"github.com/gabor-boros/minutes/internal/pkg/worklog"
	"github.com/stretchr/testify/assert"
)

func getTestEntry() worklog.Entry {
	start := time.Date(2021, 10, 2, 5, 0, 0, 0, time.UTC)
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

func TestEntry_SplitDuration(t *testing.T) {
	var splitBillable time.Duration
	var splitUnbillable time.Duration
	entry := getTestEntry()

	splitBillable, splitUnbillable = entry.SplitDuration(1)
	assert.Equal(t, entry.BillableDuration, splitBillable)
	assert.Equal(t, entry.UnbillableDuration, splitUnbillable)

	entry.UnbillableDuration = time.Hour * 2
	splitBillable, splitUnbillable = entry.SplitDuration(2)
	assert.Equal(t, time.Hour*1, splitBillable)
	assert.Equal(t, time.Hour*1, splitUnbillable)
}

func TestEntry_SplitByTag(t *testing.T) {
	entry := getTestEntry()

	regex, err := regexp.Compile(`^TASK-\d+$`)
	require.Nil(t, err)

	expectedEntries := []worklog.Entry{
		{
			Client:  entry.Client,
			Project: entry.Project,
			Task: worklog.IDNameField{
				ID:   "123",
				Name: "TASK-123",
			},
			Summary:            "test summary",
			Notes:              entry.Notes,
			Start:              entry.Start,
			BillableDuration:   entry.BillableDuration / 2,
			UnbillableDuration: entry.UnbillableDuration / 2,
		},
		{
			Client:  entry.Client,
			Project: entry.Project,
			Task: worklog.IDNameField{
				ID:   "789",
				Name: "TASK-789",
			},
			Summary:            "test summary",
			Notes:              entry.Notes,
			Start:              entry.Start,
			BillableDuration:   entry.BillableDuration / 2,
			UnbillableDuration: entry.UnbillableDuration / 2,
		},
	}

	entries := entry.SplitByTagsAsTasks("test summary", regex, []worklog.IDNameField{
		{
			ID:   "123",
			Name: "TASK-123",
		},
		{
			ID:   "456",
			Name: "NO-MATCH",
		},
		{
			ID:   "789",
			Name: "TASK-789",
		},
	})

	assert.ElementsMatch(t, expectedEntries, entries)
}
