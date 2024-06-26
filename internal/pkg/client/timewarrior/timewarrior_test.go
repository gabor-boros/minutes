package timewarrior_test

import (
	"context"
	"fmt"
	"os"
	"os/exec"
	"regexp"
	"strconv"
	"testing"
	"time"

	"github.com/gabor-boros/minutes/internal/pkg/client"
	"github.com/gabor-boros/minutes/internal/pkg/client/timewarrior"
	"github.com/gabor-boros/minutes/internal/pkg/utils"
	"github.com/gabor-boros/minutes/internal/pkg/worklog"
	"github.com/stretchr/testify/require"
)

var (
	mockedExitCode int
	mockedStdout   string
)

func mockedExecCommand(_ context.Context, command string, args ...string) *exec.Cmd {
	arguments := []string{"-test.run=TestExecCommandHelper", "--", command}
	arguments = append(arguments, args...)
	cmd := exec.Command(os.Args[0], arguments...)

	cmd.Env = []string{"GO_TEST_HELPER_PROCESS=1",
		"STDOUT=" + mockedStdout,
		"EXIT_CODE=" + strconv.Itoa(mockedExitCode),
	}

	return cmd
}

// TestExecCommandHelper is a helper test case that will be called by `mockedExecCommand`.
// This workaround is needed to be able to "mock" system calls.
func TestExecCommandHelper(t *testing.T) {
	// Not executed by the mocked command function, so return
	if os.Getenv("GO_TEST_HELPER_PROCESS") != "1" {
		return
	}

	_, _ = fmt.Fprint(os.Stdout, os.Getenv("STDOUT"))
	exitCode, err := strconv.Atoi(os.Getenv("EXIT_CODE"))
	require.NoError(t, err)

	os.Exit(exitCode)
}

func TestTimewarriorClient_FetchEntries(t *testing.T) {
	start, _ := time.ParseInLocation(utils.DateFormatRFC3339Compact.String(), "20211012T054408Z", time.Local)
	end, _ := time.ParseInLocation(utils.DateFormatRFC3339Compact.String(), "20211012T054420Z", time.Local)

	mockedExitCode = 0
	mockedStdout = `[
		{"id":3,"start":"20211012T054408Z","end":"20211012T054420Z","tags":["TASK-123","project","otherclient"],"annotation":"working on timewarrior integration"},
		{"id":2,"start":"20211012T054408Z","end":"20211012T054420Z","tags":["TASK-123","project","client","unbillable"],"annotation":"working unbilled"},
		{"id":1,"start":"20211012T054408Z","end":"20211012T054420Z","tags":["TASK-123","TASK-456","project","client","unbillable"],"annotation":"working unbilled"}
	]`

	expectedEntries := worklog.Entries{
		{
			Client: worklog.IDNameField{
				ID:   "otherclient",
				Name: "otherclient",
			},
			Project: worklog.IDNameField{
				ID:   "project",
				Name: "project",
			},
			Task: worklog.IDNameField{
				ID:   "working on timewarrior integration",
				Name: "working on timewarrior integration",
			},
			Summary:            "working on timewarrior integration",
			Notes:              "working on timewarrior integration",
			Start:              start,
			BillableDuration:   end.Sub(start),
			UnbillableDuration: 0,
		},
		{
			Client: worklog.IDNameField{
				ID:   "client",
				Name: "client",
			},
			Project: worklog.IDNameField{
				ID:   "project",
				Name: "project",
			},
			Task: worklog.IDNameField{
				ID:   "working unbilled",
				Name: "working unbilled",
			},
			Summary:            "working unbilled",
			Notes:              "working unbilled",
			Start:              start,
			BillableDuration:   0,
			UnbillableDuration: end.Sub(start),
		},
		{
			Client: worklog.IDNameField{
				ID:   "client",
				Name: "client",
			},
			Project: worklog.IDNameField{
				ID:   "project",
				Name: "project",
			},
			Task: worklog.IDNameField{
				ID:   "working unbilled",
				Name: "working unbilled",
			},
			Summary:            "working unbilled",
			Notes:              "working unbilled",
			Start:              start,
			BillableDuration:   0,
			UnbillableDuration: end.Sub(start),
		},
	}

	timewarriorClient, err := timewarrior.NewFetcher(&timewarrior.ClientOpts{
		BaseClientOpts: client.BaseClientOpts{
			Timeout: client.DefaultRequestTimeout,
		},
		CLIClient: client.CLIClient{
			Command:            "timewarrior-command",
			CommandArguments:   []string{},
			CommandCtxExecutor: mockedExecCommand,
		},
		UnbillableTag:   "unbillable",
		ClientTagRegex:  "^(client|otherclient)$",
		ProjectTagRegex: "^(project)$",
	})

	require.Nil(t, err)

	entries, err := timewarriorClient.FetchEntries(context.Background(), &client.FetchOpts{
		Start:            start,
		End:              end,
		TagsAsTasksRegex: regexp.MustCompile(""),
	})

	require.Nil(t, err, "cannot fetch entries")
	require.ElementsMatch(t, expectedEntries, entries, "fetched entries are not matching")
}

func TestTimewarriorClient_FetchEntries_TagsAsTasksRegex_NoSplit(t *testing.T) {
	start, _ := time.ParseInLocation(utils.DateFormatRFC3339Compact.String(), "20211012T054408Z", time.Local)
	end, _ := time.ParseInLocation(utils.DateFormatRFC3339Compact.String(), "20211012T054420Z", time.Local)

	mockedExitCode = 0
	mockedStdout = `[
		{"id":3,"start":"20211012T054408Z","end":"20211012T054420Z","tags":["TASK-123","project","otherclient"],"annotation":"working on timewarrior integration"},
		{"id":2,"start":"20211012T054408Z","end":"20211012T054420Z","tags":["TASK-123","project","client","unbillable"],"annotation":"working unbilled"},
		{"id":1,"start":"20211012T054408Z","end":"20211012T054420Z","tags":["TASK-456","project","client","unbillable"],"annotation":"working unbilled"}
	]`

	expectedEntries := worklog.Entries{
		{
			Client: worklog.IDNameField{
				ID:   "otherclient",
				Name: "otherclient",
			},
			Project: worklog.IDNameField{
				ID:   "project",
				Name: "project",
			},
			Task: worklog.IDNameField{
				ID:   "TASK-123",
				Name: "TASK-123",
			},
			Summary:            "working on timewarrior integration",
			Notes:              "working on timewarrior integration",
			Start:              start,
			BillableDuration:   end.Sub(start),
			UnbillableDuration: 0,
		},
		{
			Client: worklog.IDNameField{
				ID:   "client",
				Name: "client",
			},
			Project: worklog.IDNameField{
				ID:   "project",
				Name: "project",
			},
			Task: worklog.IDNameField{
				ID:   "TASK-123",
				Name: "TASK-123",
			},
			Summary:            "working unbilled",
			Notes:              "working unbilled",
			Start:              start,
			BillableDuration:   0,
			UnbillableDuration: end.Sub(start),
		},
		{
			Client: worklog.IDNameField{
				ID:   "client",
				Name: "client",
			},
			Project: worklog.IDNameField{
				ID:   "project",
				Name: "project",
			},
			Task: worklog.IDNameField{
				ID:   "TASK-456",
				Name: "TASK-456",
			},
			Summary:            "working unbilled",
			Notes:              "working unbilled",
			Start:              start,
			BillableDuration:   0,
			UnbillableDuration: end.Sub(start),
		},
	}

	timewarriorClient, err := timewarrior.NewFetcher(&timewarrior.ClientOpts{
		BaseClientOpts: client.BaseClientOpts{
			Timeout: client.DefaultRequestTimeout,
		},
		CLIClient: client.CLIClient{
			Command:            "timewarrior-command",
			CommandArguments:   []string{},
			CommandCtxExecutor: mockedExecCommand,
		},
		UnbillableTag:   "unbillable",
		ClientTagRegex:  "^(client|otherclient)$",
		ProjectTagRegex: "^(project)$",
	})

	require.Nil(t, err)

	entries, err := timewarriorClient.FetchEntries(context.Background(), &client.FetchOpts{
		Start:            start,
		End:              end,
		TagsAsTasksRegex: regexp.MustCompile(`^TASK-\d+$`),
	})

	require.Nil(t, err, "cannot fetch entries")
	require.ElementsMatch(t, expectedEntries, entries, "fetched entries are not matching")
}

func TestTimewarriorClient_FetchEntries_TagsAsTasks(t *testing.T) {
	start, _ := time.ParseInLocation(utils.DateFormatRFC3339Compact.String(), "20211012T054408Z", time.Local)
	end, _ := time.ParseInLocation(utils.DateFormatRFC3339Compact.String(), "20211012T054420Z", time.Local)

	mockedExitCode = 0
	mockedStdout = `[
		{"id":3,"start":"20211012T054408Z","end":"20211012T054420Z","tags":["TASK-123","project","otherclient"],"annotation":"working on timewarrior integration"},
		{"id":2,"start":"20211012T054408Z","end":"20211012T054420Z","tags":["TASK-123","project","client","unbillable"],"annotation":"working unbilled"},
		{"id":1,"start":"20211012T054408Z","end":"20211012T054420Z","tags":["TASK-123","TASK-456","project","client","unbillable"],"annotation":"working unbilled split"}
	]`

	expectedEntries := worklog.Entries{
		{
			Client: worklog.IDNameField{
				ID:   "otherclient",
				Name: "otherclient",
			},
			Project: worklog.IDNameField{
				ID:   "project",
				Name: "project",
			},
			Task: worklog.IDNameField{
				ID:   "TASK-123",
				Name: "TASK-123",
			},
			Summary:            "working on timewarrior integration",
			Notes:              "working on timewarrior integration",
			Start:              start,
			BillableDuration:   end.Sub(start),
			UnbillableDuration: 0,
		},
		{
			Client: worklog.IDNameField{
				ID:   "client",
				Name: "client",
			},
			Project: worklog.IDNameField{
				ID:   "project",
				Name: "project",
			},
			Task: worklog.IDNameField{
				ID:   "TASK-123",
				Name: "TASK-123",
			},
			Summary:            "working unbilled",
			Notes:              "working unbilled",
			Start:              start,
			BillableDuration:   0,
			UnbillableDuration: end.Sub(start),
		},
		{
			Client: worklog.IDNameField{
				ID:   "client",
				Name: "client",
			},
			Project: worklog.IDNameField{
				ID:   "project",
				Name: "project",
			},
			Task: worklog.IDNameField{
				ID:   "TASK-123",
				Name: "TASK-123",
			},
			Summary:            "working unbilled split",
			Notes:              "working unbilled split",
			Start:              start,
			BillableDuration:   0,
			UnbillableDuration: end.Sub(start) / 2,
		},
		{
			Client: worklog.IDNameField{
				ID:   "client",
				Name: "client",
			},
			Project: worklog.IDNameField{
				ID:   "project",
				Name: "project",
			},
			Task: worklog.IDNameField{
				ID:   "TASK-456",
				Name: "TASK-456",
			},
			Summary:            "working unbilled split",
			Notes:              "working unbilled split",
			Start:              start,
			BillableDuration:   0,
			UnbillableDuration: end.Sub(start) / 2,
		},
	}

	timewarriorClient, err := timewarrior.NewFetcher(&timewarrior.ClientOpts{
		BaseClientOpts: client.BaseClientOpts{
			Timeout: client.DefaultRequestTimeout,
		},
		CLIClient: client.CLIClient{
			Command:            "timewarrior-command",
			CommandArguments:   []string{},
			CommandCtxExecutor: mockedExecCommand,
		},
		UnbillableTag:   "unbillable",
		ClientTagRegex:  "^(client|otherclient)$",
		ProjectTagRegex: "^(project)$",
	})

	require.Nil(t, err)

	entries, err := timewarriorClient.FetchEntries(context.Background(), &client.FetchOpts{
		Start:            start,
		End:              end,
		TagsAsTasksRegex: regexp.MustCompile(`^TASK-\d+$`),
	})

	require.Nil(t, err, "cannot fetch entries")
	require.ElementsMatch(t, expectedEntries, entries, "fetched entries are not matching")
}
