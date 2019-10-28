SELECT
    attrs.path, GROUP_CONCAT(tags.name)
FROM
    attr_tags
    INNER JOIN attrs ON attr_tags.attr_id = attrs.id
    INNER JOIN tags ON attr_tags.tag_id = tags.id
GROUP BY
    attr_tags.attr_id
;
