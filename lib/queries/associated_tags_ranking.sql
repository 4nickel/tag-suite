select
    count(*) as ranking, ltags.name, rtags.name
from
    file_tags as l
    inner join
    file_tags as r
    on l.file_id == r.file_id and l.tag_id != r.tag_id
    inner join tags as ltags on ltags.id = l.tag_id
    inner join tags as rtags on rtags.id = r.tag_id
where
    (ltags.name != 'tdb::api::Entity') and
    (rtags.name != 'tdb::api::Entity') and
    (ltags.name not like 'Dataset::%') and
    (rtags.name not like 'Dataset::%') and
    (ltags.name not like 'Library::%') and
    (rtags.name not like 'Library::%') and
    (ltags.name not like 'Status::%') and
    (rtags.name not like 'Status::%') and
    (ltags.name not like 'Scope::%') and
    (rtags.name not like 'Scope::%') and
    (ltags.name not like 'Mime::%') and
    (rtags.name not like 'Mime::%') and
    (ltags.name not like 'Image::%') and
    (rtags.name not like 'Image::%')
group by
    l.tag_id, r.tag_id
order by
    l.tag_id, ranking
;
