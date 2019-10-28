select
    count(rtags.name) as rtags_count, ltags.name, group_concat(rtags.name)
from
    (select distinct l.tag_id as ltag_id, r.tag_id as rtag_id
    from
        attr_tags as l
        inner join
        attr_tags as r
        on l.attr_id == r.attr_id and l.tag_id != r.tag_id)
    inner join tags as ltags on ltags.id = ltag_id
    inner join tags as rtags on rtags.id = rtag_id
group by
    ltags.id
order by
    rtags_count
;
