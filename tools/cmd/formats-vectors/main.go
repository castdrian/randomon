package main

import (
	"os"
	"path/filepath"

	"github.com/castdrian/randomon/tools/internal/repo"
)

func main() {
	work := filepath.Join(os.TempDir(), "randomon-formats-vectors")
	output := repo.Path("crates", "randomon_formats", "tests", "vectors")
	pkrandom := filepath.Join(repo.UpstreamSource(), "com", "dabomstew", "pkrandom")
	if err := os.RemoveAll(work); err != nil {
		repo.Fail(err)
	}
	directories := map[string]string{}
	for _, name := range []string{"classes", "resources", "testpatches", "run"} {
		directories[name] = filepath.Join(work, name)
		if err := os.MkdirAll(directories[name], 0o755); err != nil {
			repo.Fail(err)
		}
	}
	sources, err := repo.JavaSources(repo.UpstreamSource(), repo.Path("tools", "vectors", "formats", "src"))
	if err != nil {
		repo.Fail(err)
	}
	compile := append([]string{"-nowarn", "-encoding", "UTF-8", "-d", directories["classes"]}, sources...)
	if err := repo.Run(repo.Root(), "javac", compile...); err != nil {
		repo.Fail(err)
	}
	resources := filepath.Join(directories["resources"], "com", "dabomstew", "pkrandom")
	for _, folder := range []string{"patches", "config"} {
		if err := os.CopyFS(filepath.Join(resources, folder), os.DirFS(filepath.Join(pkrandom, folder))); err != nil {
			repo.Fail(err)
		}
	}
	classPath := repo.ClassPath(directories["classes"], directories["resources"], directories["testpatches"])
	err = repo.Run(repo.Root(), "java", "-Djava.security.manager=allow", "-Djava.awt.headless=true",
		"-cp", classPath, "FormatsVectorGenerator", output, directories["run"], directories["testpatches"],
		filepath.Join(pkrandom, "patches"))
	if err != nil {
		repo.Fail(err)
	}
}
