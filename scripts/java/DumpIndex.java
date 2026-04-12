import java.io.*;
import java.nio.file.*;
import java.util.Map;

import org.apache.maven.index.reader.ChunkReader;
import org.apache.maven.index.reader.Record;
import org.apache.maven.index.reader.RecordExpander;

/**
 * Minimal wrapper around the Maven indexer-reader library that dumps artifact
 * records as pipe-delimited lines to stdout, one per record.
 *
 * <p>Add records:    groupId|artifactId|version|classifier|extension
 * <p>Remove records: DEL|groupId|artifactId|version|classifier|extension
 *
 * <p>Summary stats are printed to stderr.
 *
 * <p>Usage: java -cp .:lib/* DumpIndex path/to/index.gz
 */
public class DumpIndex {

    public static void main(String[] args) throws Exception {
        if (args.length != 1) {
            System.err.println("Usage: java DumpIndex <path-to-index.gz>");
            System.exit(1);
        }

        Path input = Paths.get(args[0]);
        if (!Files.isRegularFile(input)) {
            System.err.println("error: file not found: " + input);
            System.exit(1);
        }

        long adds = 0;
        long removes = 0;
        long descriptor = 0;
        long allGroups = 0;
        long rootGroups = 0;
        long total = 0;

        RecordExpander expander = new RecordExpander();
        PrintWriter out = new PrintWriter(new BufferedOutputStream(System.out));

        long startMs = System.currentTimeMillis();

        try (ChunkReader reader = new ChunkReader("input", Files.newInputStream(input))) {
            for (Map<String, String> raw : reader) {
                total++;
                Record record = expander.apply(raw);
                Record.Type type = record.getType();

                if (type == Record.Type.ARTIFACT_ADD) {
                    adds++;
                    String g = record.getString(Record.GROUP_ID);
                    String a = record.getString(Record.ARTIFACT_ID);
                    String v = record.getString(Record.VERSION);
                    String c = record.getString(Record.CLASSIFIER);
                    String e = record.getString(Record.FILE_EXTENSION);
                    out.println(
                        g + "|" + a + "|" + v + "|" +
                        (c != null ? c : "NA") + "|" +
                        (e != null ? e : "NA"));
                } else if (type == Record.Type.ARTIFACT_REMOVE) {
                    removes++;
                    String g = record.getString(Record.GROUP_ID);
                    String a = record.getString(Record.ARTIFACT_ID);
                    String v = record.getString(Record.VERSION);
                    String c = record.getString(Record.CLASSIFIER);
                    String e = record.getString(Record.FILE_EXTENSION);
                    out.println(
                        "DEL|" + g + "|" + a + "|" + v + "|" +
                        (c != null ? c : "NA") + "|" +
                        (e != null ? e : "NA"));
                } else if (type == Record.Type.DESCRIPTOR) {
                    descriptor++;
                } else if (type == Record.Type.ALL_GROUPS) {
                    allGroups++;
                } else if (type == Record.Type.ROOT_GROUPS) {
                    rootGroups++;
                }
            }
        }

        out.flush();
        long elapsedMs = System.currentTimeMillis() - startMs;

        System.err.println("parsed " + total + " documents in " + elapsedMs + "ms");
        System.err.println("  adds:       " + adds);
        System.err.println("  removes:    " + removes);
        System.err.println("  descriptor: " + descriptor);
        System.err.println("  allGroups:  " + allGroups);
        System.err.println("  rootGroups: " + rootGroups);
    }
}
