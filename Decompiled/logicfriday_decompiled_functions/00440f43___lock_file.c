/* 00440f43 __lock_file */

/* Library Function - Single Match
    __lock_file
   
   Library: Visual Studio 2003 Release */

void __cdecl __lock_file(FILE *_File)

{
  if (((FILE *)((int)&DAT_00451a44 + 3U) < _File) && (_File < (FILE *)0x451ca9)) {
    __lock(((int)&_File[-0x228d3]._bufsiz >> 5) + 0x10);
    return;
  }
  EnterCriticalSection((LPCRITICAL_SECTION)(_File + 1));
  return;
}
