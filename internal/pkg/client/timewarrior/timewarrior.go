package timewarrior

import (
	"context"
	"encoding/json"
	"fmt"
	"os/exec"
	"regexp"
	"time"

	"github.com/gabor-boros/minutes/internal/pkg/client"
	"github.com/gabor-boros/minutes/internal/pkg/worklog"
)

const (
	dateFormat      string = "2006-01-02T15:04:05"
	ParseDateFormat string = "20060102T150405Z"
)

// FetchEntry represents the entry exported from Timewarrior.
type FetchEntry struct {
	ID         int      `json:"id"`
	Start      string   `json:"start"`
	End        string   `json:"end"`
	Tags       []string `json:"tags"`
	Annotation string   `json:"annotation"`
}

// ClientOpts is the client specific options, extending client.BaseClientOpts.
// Since Timewarrior is a CLI tool, hence it has no API we could call on HTTP.
// Although client.HTTPClientOpts is part of client.BaseClientOpts, we are
// not using that as part of this integration, instead we are defining the path
// of the executable (Command) and the command arguments used for export
// (CommandArguments).
type ClientOpts struct {
	client.BaseClientOpts
	Command            string
	CommandArguments   []string
	CommandCtxExecutor func(ctx context.Context, name string, arg ...string) *exec.Cmd
	UnbillableTag      string
	ClientTagRegex     string
	ProjectTagRegex    string
}

type timewarriorClient struct {
	opts *ClientOpts
}

func (c *timewarriorClient) assembleCommand(subcommand string, opts *client.FetchOpts) (string, []string) {
	arguments := []string{subcommand}

	arguments = append(
		arguments,
		[]string{
			"from", opts.Start.Format(dateFormat),
			"to", opts.End.Format(dateFormat),
		}...,
	)

	arguments = append(arguments, c.opts.CommandArguments...)

	return c.opts.Command, arguments
}

func (c *timewarriorClient) FetchEntries(ctx context.Context, opts *client.FetchOpts) ([]worklog.Entry, error) {
	var fetchedEntries []FetchEntry

	command, arguments := c.assembleCommand("export", opts)

	out, err := c.opts.CommandCtxExecutor(ctx, command, arguments...).Output() // #nosec G204
	if err != nil {
		return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	if err = json.Unmarshal(out, &fetchedEntries); err != nil {
		return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	clientTagRegex, err := regexp.Compile(c.opts.ClientTagRegex)
	if err != nil {
		return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	projectTagRegex, err := regexp.Compile(c.opts.ProjectTagRegex)
	if err != nil {
		return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	tagsAsTasksRegex, err := regexp.Compile(c.opts.TagsAsTasksRegex)
	if err != nil {
		return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	var entries []worklog.Entry
	for _, entry := range fetchedEntries {
		var clientName string
		var projectName string
		var task worklog.IDNameField

		startDate, err := time.ParseInLocation(ParseDateFormat, entry.Start, time.Local)
		if err != nil {
			return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
		}

		endDate, err := time.ParseInLocation(ParseDateFormat, entry.End, time.Local)
		if err != nil {
			return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
		}

		billableDuration := endDate.Sub(startDate)
		unbillableDuration := time.Duration(0)

		for _, tag := range entry.Tags {
			if tag == c.opts.UnbillableTag {
				unbillableDuration = billableDuration
				billableDuration = 0
			} else if c.opts.ClientTagRegex != "" && clientTagRegex.MatchString(tag) {
				clientName = tag
			} else if c.opts.ProjectTagRegex != "" && projectTagRegex.MatchString(tag) {
				projectName = tag
			} else if c.opts.TagsAsTasksRegex != "" && tagsAsTasksRegex.MatchString(tag) {
				task = worklog.IDNameField{
					ID:   tag,
					Name: tag,
				}
			}
		}

		// If the task was not found in tags, make sure to set it to annotation
		if !task.IsComplete() {
			task = worklog.IDNameField{
				ID:   entry.Annotation,
				Name: entry.Annotation,
			}
		}

		worklogEntry := worklog.Entry{
			Client: worklog.IDNameField{
				ID:   clientName,
				Name: clientName,
			},
			Project: worklog.IDNameField{
				ID:   projectName,
				Name: projectName,
			},
			Task:               task,
			Summary:            entry.Annotation,
			Notes:              entry.Annotation,
			Start:              startDate,
			BillableDuration:   billableDuration,
			UnbillableDuration: unbillableDuration,
		}

		if c.opts.TagsAsTasks && len(entry.Tags) > 0 {
			var tags []worklog.IDNameField
			for _, tag := range entry.Tags {
				tags = append(tags, worklog.IDNameField{
					ID:   tag,
					Name: tag,
				})
			}

			splitEntries := worklogEntry.SplitByTagsAsTasks(worklogEntry.Summary, tagsAsTasksRegex, tags)
			entries = append(entries, splitEntries...)
		} else {
			entries = append(entries, worklogEntry)
		}
	}

	return entries, nil
}

// NewClient returns a new Timewarrior client.
func NewClient(opts *ClientOpts) client.Fetcher {
	return &timewarriorClient{
		opts: opts,
	}
}
