create table expressions (
	expression_hash blob primary key,
	expression blob not null,
	value blob not null
);

create table artifacts (
	artifact_hash blob primary key,
	foreign key (artifact_hash) references objects (object_hash)
);

create table artifact_dependencies (
	artifact_hash blob not null,
	dependency_hash blob not null,
	foreign key (artifact_hash) references artifacts (artifact_hash),
	foreign key (dependency_hash) references artifacts (artifact_hash)
);
create index artifact_dependencies_hash_index on artifact_dependencies (artifact_hash);
create index artifact_dependencies_dependency_hash_index on artifact_dependencies (dependency_hash);

create table if not exists objects (
	object_hash blob primary key,
	object_data blob not null
);

create table roots (
	artifact_hash blob primary key,
	foreign key (artifact_hash) references artifacts (artifact_hash)
);
