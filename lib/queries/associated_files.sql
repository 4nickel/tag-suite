select
    count(rfiles.path) as rfiles_count, lfiles.path
from
    (select distinct l.file_id as lfile_id, r.file_id as rfile_id
    from
        file_tags as l
        inner join
        file_tags as r
        on l.tag_id == r.tag_id and l.file_id != r.file_id)
    inner join files as lfiles on lfiles.id = lfile_id
    inner join files as rfiles on rfiles.id = rfile_id
group by
    lfile_id
order by
    rfiles_count
;
