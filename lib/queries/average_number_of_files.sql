SELECT
    AVG(file_count)
FROM
(
    SELECT
        COUNT(files.id) AS file_count
    FROM
        file_tags
        INNER JOIN files ON file_tags.file_id = files.id
        INNER JOIN tags  ON file_tags.tag_id = tags.id
    GROUP BY
        tags.id
)
