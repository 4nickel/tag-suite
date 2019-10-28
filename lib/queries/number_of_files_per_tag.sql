SELECT
    COUNT(attrs.id) AS attr_count, tags.name
FROM
    attr_tags
    INNER JOIN attrs ON attr_tags.attr_id = attrs.id
    INNER JOIN tags  ON attr_tags.tag_id = tags.id
GROUP BY
    tags.id
ORDER BY
    attr_count
