select
    count(*) as ranking, lfiles.path, rfiles.path
from
    file_tags as l
    inner join
    file_tags as r
    on l.tag_id == r.tag_id and l.file_id != r.file_id
    inner join files as lfiles on lfiles.id = l.file_id
    inner join files as rfiles on rfiles.id = r.file_id
group by
    l.file_id, r.file_id
order by
    ranking
;
