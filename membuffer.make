
$(OBJDIR)membuffer$(OBJ_SUFFIX): membuffer.cpp makefile.rules
	$(CXX) $(TOOL_CXXFLAGS) $(COMP_OBJ)$@ $<

$(OBJDIR)membuffer$(PINTOOL_SUFFIX): $(OBJDIR)membuffer$(OBJ_SUFFIX)
	$(LINKER) $(TOOL_LDFLAGS) $(LINK_EXE)$@ $^ $(TOOL_LPATHS) $(TOOL_LIBS) libz.a
