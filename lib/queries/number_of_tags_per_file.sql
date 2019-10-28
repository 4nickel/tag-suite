SELECT
    COUNT(tags.id) AS tag_count, attrs.path
FROM
    attr_tags
    INNER JOIN attrs ON attr_tags.attr_id = attrs.id
    INNER JOIN tags  ON attr_tags.tag_id = tags.id
GROUP BY
    attrs.id
ORDER BY
    tag_count
