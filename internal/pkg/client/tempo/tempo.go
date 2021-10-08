package tempo

import (
	"context"
	"encoding/json"
	"fmt"
	"math"
	"net/http"
	"strconv"
	"time"

	"github.com/gabor-boros/minutes/internal/pkg/client"
	"github.com/gabor-boros/minutes/internal/pkg/worklog"
)

const (
	// PathWorklogCreate is the endpoint used to create new worklogs.
	PathWorklogCreate string = "/rest/tempo-timesheets/4/worklogs"
	// PathWorklogSearch is the endpoint used to search existing worklogs.
	PathWorklogSearch string = "/rest/tempo-timesheets/4/worklogs/search"
)

// Issue represents the Jira issue the time logged against.
type Issue struct {
	ID         int    `json:"id"`
	Key        string `json:"key"`
	AccountKey string `json:"accountKey"`
	ProjectID  int    `json:"projectId"`
	ProjectKey string `json:"projectKey"`
	Summary    string `json:"summary"`
}

// FetchEntry represents the entry fetched from Tempo.
// StartDate must be in the given YYYY-MM-DD format, required by Tempo.
type FetchEntry struct {
	ID               int       `json:"id"`
	StartDate        time.Time `json:"startDate"`
	BillableSeconds  int       `json:"billableSeconds"`
	TimeSpentSeconds int       `json:"timeSpentSeconds"`
	Comment          string    `json:"comment"`
	WorkerKey        string    `json:"workerKey"`
	Issue            Issue     `json:"issue"`
}

// UploadEntry represents the payload to create a new worklog in Tempo.
// Started must be in the given YYYY-MM-DD format, required by Tempo.
type UploadEntry struct {
	Comment               string `json:"comment,omitempty"`
	IncludeNonWorkingDays bool   `json:"includeNonWorkingDays,omitempty"`
	OriginTaskID          string `json:"originTaskId,omitempty"`
	Started               string `json:"started,omitempty"`
	BillableSeconds       int    `json:"billableSeconds,omitempty"`
	TimeSpentSeconds      int    `json:"timeSpentSeconds,omitempty"`
	Worker                string `json:"worker,omitempty"`
}

// SearchParams represents the parameters used to filter Tempo search results.
// From and To must be in the given YYYY-MM-DD format, required by Tempo.
type SearchParams struct {
	From   string `json:"from"`
	To     string `json:"to"`
	Worker string `json:"worker"`
}

// ClientOpts is the client specific options, extending client.BaseClientOpts.
type ClientOpts struct {
	client.BaseClientOpts
}

type tempoClient struct {
	opts *ClientOpts
}

func (c *tempoClient) FetchEntries(ctx context.Context, opts *client.FetchOpts) ([]worklog.Entry, error) {
	searchParams := &SearchParams{
		From:   opts.Start.Local().Format("2006-01-02"),
		To:     opts.End.Local().Format("2006-01-02"),
		Worker: opts.User,
	}

	resp, err := client.SendRequest(ctx, http.MethodPost, PathWorklogSearch, searchParams, &c.opts.HTTPClientOptions)
	if err != nil {
		return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	var fetchedEntries []FetchEntry
	if err = json.NewDecoder(resp.Body).Decode(&fetchedEntries); err != nil {
		return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	var entries []worklog.Entry
	for _, entry := range fetchedEntries {
		entries = append(entries, worklog.Entry{
			Client: worklog.IDNameField{
				ID:   entry.Issue.AccountKey,
				Name: entry.Issue.AccountKey,
			},
			Project: worklog.IDNameField{
				ID:   strconv.Itoa(entry.Issue.ProjectID),
				Name: entry.Issue.ProjectKey,
			},
			Task: worklog.IDNameField{
				ID:   strconv.Itoa(entry.Issue.ID),
				Name: entry.Issue.Key,
			},
			Summary:            entry.Issue.Summary,
			Notes:              entry.Comment,
			Start:              entry.StartDate,
			BillableDuration:   time.Second * time.Duration(entry.BillableSeconds),
			UnbillableDuration: time.Second * time.Duration(entry.TimeSpentSeconds-entry.BillableSeconds),
		})
	}

	return entries, nil
}

func (c *tempoClient) uploadEntry(ctx context.Context, entry worklog.Entry, opts *client.UploadOpts, errChan chan error) {
	billableDuration := entry.BillableDuration
	unbillableDuration := entry.UnbillableDuration
	totalTimeSpent := billableDuration + unbillableDuration

	if opts.TreatDurationAsBilled {
		billableDuration = entry.UnbillableDuration + entry.BillableDuration
		unbillableDuration = 0
	}

	if opts.RoundToClosestMinute {
		billableDuration = time.Second * time.Duration(math.Round(billableDuration.Minutes())*60)
		unbillableDuration = time.Second * time.Duration(math.Round(unbillableDuration.Minutes())*60)
		totalTimeSpent = billableDuration + unbillableDuration
	}

	uploadEntry := &UploadEntry{
		Comment:               entry.Summary,
		IncludeNonWorkingDays: true,
		OriginTaskID:          entry.Task.Name,
		Started:               entry.Start.Local().Format("2006-01-02"),
		BillableSeconds:       int(billableDuration.Seconds()),
		TimeSpentSeconds:      int(totalTimeSpent.Seconds()),
		Worker:                opts.User,
	}

	if _, err := client.SendRequest(ctx, http.MethodPost, PathWorklogCreate, uploadEntry, &c.opts.HTTPClientOptions); err != nil {
		errChan <- err
		return
	}

	errChan <- nil
}

func (c *tempoClient) UploadEntries(ctx context.Context, entries []worklog.Entry, opts *client.UploadOpts) error {
	errChan := make(chan error)

	for _, entry := range entries {
		go c.uploadEntry(ctx, entry, opts, errChan)
	}

	for i := 0; i < len(entries); i++ {
		if err := <-errChan; err != nil {
			return fmt.Errorf("%v: %v", client.ErrUploadEntries, err)
		}
	}

	return nil
}

// NewClient returns a new Tempo client.
func NewClient(opts *ClientOpts) client.FetchUploader {
	return &tempoClient{
		opts: opts,
	}
}
