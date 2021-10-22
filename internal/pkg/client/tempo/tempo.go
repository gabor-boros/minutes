package tempo

import (
	"context"
	"encoding/json"
	"fmt"
	"math"
	"net/http"
	"net/url"
	"strconv"
	"time"

	"github.com/gabor-boros/minutes/internal/pkg/client"
	"github.com/gabor-boros/minutes/internal/pkg/utils"
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
	client.BasicAuth
	BaseURL string
}

type tempoClient struct {
	*client.BaseClientOpts
	*client.HTTPClient
	*client.DefaultUploader
	authenticator client.Authenticator
}

func (c *tempoClient) FetchEntries(ctx context.Context, opts *client.FetchOpts) (worklog.Entries, error) {
	searchURL, err := c.URL(PathWorklogSearch, map[string]string{})
	if err != nil {
		return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	resp, err := c.Call(ctx, &client.HTTPRequestOpts{
		Method:  http.MethodPost,
		Url:     searchURL,
		Auth:    c.authenticator,
		Timeout: c.Timeout,
		Data: &SearchParams{
			From:   utils.DateFormatISO8601.Format(opts.Start.Local()),
			To:     utils.DateFormatISO8601.Format(opts.End.Local()),
			Worker: opts.User,
		},
		Headers: map[string]string{
			"Content-Type": "application/json",
		},
	})

	if err != nil {
		return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	var fetchedEntries []FetchEntry
	if err = json.Unmarshal(resp, &fetchedEntries); err != nil {
		return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	var entries worklog.Entries
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

func (c *tempoClient) UploadEntries(ctx context.Context, entries worklog.Entries, errChan chan error, opts *client.UploadOpts) {
	createURL, err := c.URL(PathWorklogCreate, map[string]string{})
	if err != nil {
		errChan <- fmt.Errorf("%v: %v", client.ErrUploadEntries, err)
		return
	}

	for _, groupEntries := range entries.GroupByTask() {
		go func(ctx context.Context, entries worklog.Entries, errChan chan error, opts *client.UploadOpts) {
			for _, entry := range entries {
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
					Started:               utils.DateFormatISO8601.Format(entry.Start.Local()),
					BillableSeconds:       int(billableDuration.Seconds()),
					TimeSpentSeconds:      int(totalTimeSpent.Seconds()),
					Worker:                opts.User,
				}

				tracker := c.StartTracking(entry, opts.ProgressWriter)

				_, err := c.Call(ctx, &client.HTTPRequestOpts{
					Method:  http.MethodPost,
					Url:     createURL,
					Auth:    c.authenticator,
					Timeout: c.Timeout,
					Data:    uploadEntry,
					Headers: map[string]string{
						"Content-Type": "application/json",
					},
				})

				if err != nil {
					err = fmt.Errorf("%v: %+v: %v", client.ErrUploadEntries, uploadEntry, err)
				}

				c.StopTracking(tracker, err)
				errChan <- err
			}
		}(ctx, groupEntries, errChan, opts)
	}
}

func newClient(opts *ClientOpts) (*tempoClient, error) {
	baseURL, err := url.Parse(opts.BaseURL)
	if err != nil {
		return nil, err
	}

	authenticator, err := client.NewBasicAuth(opts.Username, opts.Password)
	if err != nil {
		return nil, err
	}

	return &tempoClient{
		authenticator:  authenticator,
		HTTPClient:     &client.HTTPClient{BaseURL: baseURL},
		BaseClientOpts: &opts.BaseClientOpts,
	}, nil
}

// NewFetcher returns a new Tempo client for fetching entries.
func NewFetcher(opts *ClientOpts) (client.Fetcher, error) {
	return newClient(opts)
}

// NewUploader returns a new Tempo client for uploading entries.
func NewUploader(opts *ClientOpts) (client.Uploader, error) {
	return newClient(opts)
}
