SELECT
    attrs.id         AS attrs_id,
    attrs.path       AS attrs_name,
    attr_tags.tag_id AS tags_id,
    tags.name        AS tags_name
FROM
    attr_tags
    INNER JOIN attrs ON attr_tags.attr_id = attrs_id
    INNER JOIN tags  ON attr_tags.tag_id = tags_id
